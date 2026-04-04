import Foundation

/// Fluent builder for configuring and running zag agent sessions.
///
/// ```swift
/// let output = try await ZagBuilder()
///     .provider("claude")
///     .model("sonnet")
///     .autoApprove()
///     .exec("write a hello world program")
///
/// print(output.result ?? "")
/// ```
public final class ZagBuilder {

    // MARK: - Private state

    private var bin: String = ZagProcess.defaultBin
    private var _provider: String?
    private var _model: String?
    private var _systemPrompt: String?
    private var _root: String?
    private var _autoApprove = false
    private var _addDirs: [String] = []
    private var _json = false
    private var _jsonSchema: String?
    private var _jsonStream = false
    private var _worktree: IsolationOption?
    private var _sandbox: IsolationOption?
    private var _verbose = false
    private var _quiet = false
    private var _debug = false
    private var _sessionId: String?
    private var _outputFormat: String?
    private var _inputFormat: String?
    private var _replayUserMessages = false
    private var _includePartialMessages = false
    private var _maxTurns: Int?
    private var _showUsage = false
    private var _size: String?

    private enum IsolationOption {
        case enabled
        case named(String)
    }

    public init() {}

    // MARK: - Configuration methods

    /// Override the zag binary path (default: `ZAG_BIN` env or `"zag"`).
    @discardableResult
    public func bin(_ path: String) -> Self { bin = path; return self }

    /// Set the provider (e.g., `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`).
    @discardableResult
    public func provider(_ p: String) -> Self { _provider = p; return self }

    /// Set the model (e.g., `"sonnet"`, `"opus"`, `"small"`, `"large"`).
    @discardableResult
    public func model(_ m: String) -> Self { _model = m; return self }

    /// Set a system prompt to configure agent behavior.
    @discardableResult
    public func systemPrompt(_ p: String) -> Self { _systemPrompt = p; return self }

    /// Set the root directory for the agent to operate in.
    @discardableResult
    public func root(_ r: String) -> Self { _root = r; return self }

    /// Enable auto-approve mode (skip permission prompts).
    @discardableResult
    public func autoApprove() -> Self { _autoApprove = true; return self }

    /// Add an additional directory for the agent to include.
    @discardableResult
    public func addDir(_ d: String) -> Self { _addDirs.append(d); return self }

    /// Request JSON output from the agent.
    @discardableResult
    public func json() -> Self { _json = true; return self }

    /// Set a JSON schema for structured output validation. Implies `json()`.
    @discardableResult
    public func jsonSchema(_ s: String) -> Self { _jsonSchema = s; _json = true; return self }

    /// Enable streaming JSON output (NDJSON format).
    @discardableResult
    public func jsonStream() -> Self { _jsonStream = true; return self }

    /// Enable worktree mode with an optional name.
    @discardableResult
    public func worktree(_ name: String? = nil) -> Self {
        _worktree = name.map { .named($0) } ?? .enabled
        return self
    }

    /// Enable sandbox mode with an optional name.
    @discardableResult
    public func sandbox(_ name: String? = nil) -> Self {
        _sandbox = name.map { .named($0) } ?? .enabled
        return self
    }

    /// Enable verbose output.
    @discardableResult
    public func verbose() -> Self { _verbose = true; return self }

    /// Enable quiet mode.
    @discardableResult
    public func quiet() -> Self { _quiet = true; return self }

    /// Enable debug logging.
    @discardableResult
    public func debug() -> Self { _debug = true; return self }

    /// Pre-set a session ID (UUID).
    @discardableResult
    public func sessionId(_ id: String) -> Self { _sessionId = id; return self }

    /// Set the output format (e.g., `"text"`, `"json"`, `"json-pretty"`, `"stream-json"`).
    @discardableResult
    public func outputFormat(_ f: String) -> Self { _outputFormat = f; return self }

    /// Set the input format (`"text"`, `"stream-json"` — Claude only).
    @discardableResult
    public func inputFormat(_ f: String) -> Self { _inputFormat = f; return self }

    /// Re-emit user messages from stdin on stdout (Claude only).
    @discardableResult
    public func replayUserMessages() -> Self { _replayUserMessages = true; return self }

    /// Include partial message chunks in streaming output (Claude only).
    @discardableResult
    public func includePartialMessages() -> Self { _includePartialMessages = true; return self }

    /// Set the maximum number of agentic turns.
    @discardableResult
    public func maxTurns(_ n: Int) -> Self { _maxTurns = n; return self }

    /// Show token usage statistics (only applies to JSON output mode).
    @discardableResult
    public func showUsage() -> Self { _showUsage = true; return self }

    /// Set the Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`).
    @discardableResult
    public func size(_ s: String) -> Self { _size = s; return self }

    // MARK: - Arg building

    /// Build global CLI arguments shared across all commands.
    public func buildGlobalArgs() -> [String] {
        var args: [String] = []
        if let p = _provider { args += ["-p", p] }
        if let m = _model { args += ["--model", m] }
        if let s = _systemPrompt { args += ["--system-prompt", s] }
        if let r = _root { args += ["--root", r] }
        if _autoApprove { args.append("--auto-approve") }
        for d in _addDirs { args += ["--add-dir", d] }
        switch _worktree {
        case .enabled: args.append("-w")
        case .named(let n): args += ["-w", n]
        case nil: break
        }
        switch _sandbox {
        case .enabled: args.append("--sandbox")
        case .named(let n): args += ["--sandbox", n]
        case nil: break
        }
        if _verbose { args.append("--verbose") }
        if _quiet { args.append("--quiet") }
        if _debug { args.append("--debug") }
        if let id = _sessionId { args += ["--session", id] }
        if let n = _maxTurns { args += ["--max-turns", String(n)] }
        if _showUsage { args.append("--show-usage") }
        if let s = _size { args += ["--size", s] }
        return args
    }

    /// Build CLI arguments for the `exec` subcommand.
    public func buildExecArgs(prompt: String, streaming: Bool = false) -> [String] {
        var args = buildGlobalArgs()
        args.append("exec")
        if _json { args.append("--json") }
        if let s = _jsonSchema { args += ["--json-schema", s] }
        if _jsonStream || streaming { args.append("--json-stream") }
        if let f = _outputFormat { args += ["-o", f] }
        if let f = _inputFormat { args += ["-i", f] }
        if _replayUserMessages { args.append("--replay-user-messages") }
        if _includePartialMessages { args.append("--include-partial-messages") }
        // Default to json output for structured parsing
        if !streaming && _outputFormat == nil && !_jsonStream {
            args += ["-o", "json"]
        }
        args.append(prompt)
        return args
    }

    // MARK: - Terminal methods

    /// Run the agent non-interactively and return structured output.
    public func exec(_ prompt: String) async throws -> AgentOutput {
        let args = buildExecArgs(prompt: prompt)
        return try await ZagProcess.exec(bin: bin, args: args)
    }

    /// Run the agent in streaming mode, yielding events as they arrive.
    public func stream(_ prompt: String) -> AsyncThrowingStream<Event, Error> {
        let args = buildExecArgs(prompt: prompt, streaming: true)
        return ZagProcess.stream(bin: bin, args: args)
    }

    /// Run the agent with streaming input and output (Claude only).
    public func execStreaming(_ prompt: String) throws -> StreamingSession {
        var args = buildGlobalArgs()
        args.append("exec")
        args += ["-i", "stream-json"]
        args += ["-o", "stream-json"]
        args.append("--replay-user-messages")
        if _includePartialMessages { args.append("--include-partial-messages") }
        args.append(prompt)
        return try ZagProcess.startStreamingProcess(bin: bin, args: args)
    }

    /// Start an interactive agent session (inherits stdio).
    public func run(_ prompt: String? = nil) async throws {
        var args = buildGlobalArgs()
        args.append("run")
        if _json { args.append("--json") }
        if let s = _jsonSchema { args += ["--json-schema", s] }
        if let p = prompt { args.append(p) }
        try await ZagProcess.runInteractive(bin: bin, args: args)
    }

    /// Resume a previous session by ID.
    public func resume(_ sessionId: String) async throws {
        var args = buildGlobalArgs()
        args.append("run")
        args += ["--resume", sessionId]
        try await ZagProcess.runInteractive(bin: bin, args: args)
    }

    /// Resume the most recent session.
    public func continueLast() async throws {
        var args = buildGlobalArgs()
        args.append("run")
        args.append("--continue")
        try await ZagProcess.runInteractive(bin: bin, args: args)
    }
}
