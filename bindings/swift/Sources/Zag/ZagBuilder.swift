import Foundation

/// Fluent builder for configuring and running zag agent sessions.
///
/// **Local mode** (macOS/Linux — requires `zag` CLI on PATH):
/// ```swift
/// let output = try await ZagBuilder()
///     .provider("claude")
///     .model("sonnet")
///     .autoApprove()
///     .exec("write a hello world program")
///
/// print(output.result ?? "")
/// ```
///
/// **Remote mode** (macOS/iOS/Linux — requires a `zag serve` instance):
/// ```swift
/// let output = try await ZagBuilder()
///     .remote(url: "https://server:2100", token: "my-token")
///     .provider("claude")
///     .model("sonnet")
///     .autoApprove()
///     .exec("write a hello world program")
///
/// print(output.result ?? "")
/// ```
public final class ZagBuilder {

    // MARK: - Private state

    #if os(macOS) || os(Linux)
    private var bin: String = ZagProcess.defaultBin
    #else
    private var bin: String = "zag"
    #endif
    private var _provider: String?
    private var _model: String?
    private var _systemPrompt: String?
    private var _root: String?
    private var _autoApprove = false
    private var _addDirs: [String] = []
    private var _files: [String] = []
    private var _envVars: [String] = []
    private var _json = false
    private var _jsonSchema: String?
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
    private var _timeout: String?
    private var _mcpConfig: String?
    private var _showUsage = false
    private var _size: String?
    private var _connection: ZagConnection?
    private var _urlSession: URLSession?

    private enum IsolationOption {
        case enabled
        case named(String)
    }

    public init() {}

    // MARK: - Configuration methods

    /// Override the zag binary path (default: `ZAG_BIN` env or `"zag"`).
    /// Only relevant for local execution mode.
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

    /// Attach a file to the prompt (chainable).
    @discardableResult
    public func file(_ path: String) -> Self { _files.append(path); return self }

    /// Add an environment variable for the agent subprocess.
    @discardableResult
    public func env(_ key: String, _ value: String) -> Self { _envVars.append("\(key)=\(value)"); return self }

    /// Request JSON output from the agent.
    @discardableResult
    public func json() -> Self { _json = true; return self }

    /// Set a JSON schema for structured output validation. Implies `json()`.
    @discardableResult
    public func jsonSchema(_ s: String) -> Self { _jsonSchema = s; _json = true; return self }

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

    /// Set a timeout duration (e.g., `"30s"`, `"5m"`, `"1h"`). Kills the agent if exceeded.
    @discardableResult
    public func timeout(_ t: String) -> Self { _timeout = t; return self }

    /// Set MCP server config for this invocation: JSON string or file path (Claude only).
    @discardableResult
    public func mcpConfig(_ c: String) -> Self { _mcpConfig = c; return self }

    /// Show token usage statistics (only applies to JSON output mode).
    @discardableResult
    public func showUsage() -> Self { _showUsage = true; return self }

    /// Set the Ollama model parameter size (e.g., `"2b"`, `"9b"`, `"35b"`).
    @discardableResult
    public func size(_ s: String) -> Self { _size = s; return self }

    /// Configure a remote `zag serve` connection.
    /// When set, terminal methods use HTTP/WebSocket instead of local subprocess.
    @discardableResult
    public func connection(_ c: ZagConnection) -> Self { _connection = c; return self }

    /// Convenience: configure a remote `zag serve` connection from URL and token strings.
    @discardableResult
    public func remote(url: String, token: String) -> Self {
        _connection = try? ZagConnection(url: url, token: token)
        return self
    }

    /// Set a custom `URLSession` for remote requests (useful for testing).
    @discardableResult
    public func urlSession(_ s: URLSession) -> Self { _urlSession = s; return self }

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
        for f in _files { args += ["--file", f] }
        for e in _envVars { args += ["--env", e] }
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
        if let c = _mcpConfig { args += ["--mcp-config", c] }
        if _showUsage { args.append("--show-usage") }
        if let s = _size { args += ["--size", s] }
        return args
    }

    /// Build CLI arguments for the `exec` subcommand.
    public func buildExecArgs(prompt: String, streaming: Bool = false) -> [String] {
        // Subcommand must precede per-subcommand flags; only --debug/--quiet/--verbose
        // are truly global (cli-level) and could go before the subcommand, but placing
        // everything after the subcommand is equally valid in clap.
        var args: [String] = ["exec"]
        args += buildGlobalArgs()
        if _json { args.append("--json") }
        if let s = _jsonSchema { args += ["--json-schema", s] }
        if let f = _outputFormat {
            args += ["-o", f]
        } else if streaming {
            args += ["-o", "stream-json"]
        } else {
            // Default to json output for structured parsing
            args += ["-o", "json"]
        }
        if let f = _inputFormat { args += ["-i", f] }
        if _replayUserMessages { args.append("--replay-user-messages") }
        if _includePartialMessages { args.append("--include-partial-messages") }
        if let t = _timeout { args += ["--timeout", t] }
        args.append(prompt)
        return args
    }

    /// Build `SpawnParams` from builder state for remote execution.
    public func buildSpawnParams(prompt: String) -> SpawnParams {
        SpawnParams(
            prompt: prompt,
            provider: _provider,
            model: _model,
            root: _root,
            autoApprove: _autoApprove ? true : nil,
            systemPrompt: _systemPrompt,
            addDirs: _addDirs.isEmpty ? nil : _addDirs,
            size: _size,
            maxTurns: _maxTurns.map { Int($0) },
            timeout: _timeout
        )
    }

    // MARK: - Version checking

    #if os(macOS) || os(Linux)
    private func versionRequirements() -> [VersionCheck.Requirement] {
        return [
            VersionCheck.Requirement(method: "env()", version: "0.6.0", isSet: !_envVars.isEmpty),
            VersionCheck.Requirement(method: "mcpConfig()", version: "0.6.0", isSet: _mcpConfig != nil),
        ]
    }

    // MARK: - Capability checking

    /// Collect provider-capability requirements for options that are only
    /// supported by a subset of providers. Note: `mcpConfig()` is intentionally
    /// omitted — there is no `mcp_config` field on the provider `Features`
    /// struct yet, so there is nothing to validate against.
    private func capabilityRequirements() -> [CapabilityCheck.Requirement] {
        return [
            CapabilityCheck.Requirement(method: "worktree()", feature: "worktree", isSet: _worktree != nil),
            CapabilityCheck.Requirement(method: "sandbox()", feature: "sandbox", isSet: _sandbox != nil),
            CapabilityCheck.Requirement(method: "systemPrompt()", feature: "system_prompt", isSet: _systemPrompt != nil),
            CapabilityCheck.Requirement(method: "addDir()", feature: "add_dirs", isSet: !_addDirs.isEmpty),
        ]
    }
    #endif

    // MARK: - Terminal methods

    /// Run the agent non-interactively and return structured output.
    ///
    /// In remote mode, this spawns a session, waits for completion, and fetches the output
    /// via the `zag serve` HTTP API.
    public func exec(_ prompt: String) async throws -> AgentOutput {
        if let conn = _connection {
            let client = ZagRemoteClient(connection: conn, session: _urlSession ?? .shared)
            let params = buildSpawnParams(prompt: prompt)
            let spawned = try await client.spawn(params)
            _ = try await client.wait(sessionIds: [spawned.sessionId])
            let out = try await client.output(spawned.sessionId)
            // Build an AgentOutput from the remote response
            return AgentOutput(
                agent: _provider ?? "unknown",
                sessionId: spawned.sessionId,
                result: out.result,
                isError: false)
        }
        #if os(macOS) || os(Linux)
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capabilityRequirements())
        let args = buildExecArgs(prompt: prompt)
        return try await ZagProcess.exec(bin: bin, args: args)
        #else
        throw ZagError(message: "Local execution requires macOS or Linux. Use .connection() or .remote(url:token:) for remote execution.")
        #endif
    }

    /// Run the agent in streaming mode, yielding events as they arrive.
    ///
    /// In remote mode, this spawns a session and streams events via WebSocket.
    public func stream(_ prompt: String) -> AsyncThrowingStream<Event, Error> {
        if let conn = _connection {
            let client = ZagRemoteClient(connection: conn, session: _urlSession ?? .shared)
            let params = buildSpawnParams(prompt: prompt)
            return AsyncThrowingStream { continuation in
                Task {
                    do {
                        let spawned = try await client.spawn(params)
                        let eventStream = client.stream(spawned.sessionId)
                        for try await event in eventStream {
                            continuation.yield(event)
                        }
                        continuation.finish()
                    } catch {
                        continuation.finish(throwing: error)
                    }
                }
            }
        }
        #if os(macOS) || os(Linux)
        let args = buildExecArgs(prompt: prompt, streaming: true)
        let binPath = bin
        let requirements = versionRequirements()
        let capReqs = capabilityRequirements()
        let capturedProvider = _provider
        return AsyncThrowingStream { continuation in
            Task {
                do {
                    try await VersionCheck.check(bin: binPath, requirements: requirements)
                    try await CapabilityCheck.check(bin: binPath, provider: capturedProvider, requirements: capReqs)
                    let innerStream = ZagProcess.stream(bin: binPath, args: args)
                    for try await event in innerStream {
                        continuation.yield(event)
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
        #else
        return AsyncThrowingStream { continuation in
            continuation.finish(throwing: ZagError(
                message: "Local execution requires macOS or Linux. Use .connection() or .remote(url:token:) for remote execution."))
        }
        #endif
    }

    /// Run the agent with streaming input and output (Claude only).
    ///
    /// In remote mode, this spawns a session and returns a `ZagRemoteSession`
    /// backed by WebSocket for bidirectional communication.
    /// In local mode, this returns a `StreamingSession` backed by subprocess pipes.
    ///
    /// - Note: On iOS, only remote mode is available.
    #if os(macOS) || os(Linux)
    public func execStreaming(_ prompt: String) async throws -> StreamingSession {
        if _connection != nil {
            fatalError("Use execStreamingRemote(_:) for remote streaming sessions.")
        }
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        var capReqs = capabilityRequirements()
        capReqs.append(CapabilityCheck.Requirement(
            method: "execStreaming()", feature: "streaming_input", isSet: true))
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capReqs)
        var args: [String] = ["exec"]
        args += buildGlobalArgs()
        args += ["-i", "stream-json"]
        args += ["-o", "stream-json"]
        args.append("--replay-user-messages")
        if _includePartialMessages { args.append("--include-partial-messages") }
        args.append(prompt)
        return try ZagProcess.startStreamingProcess(bin: bin, args: args)
    }
    #endif

    /// Run the agent with streaming input and output via remote WebSocket.
    public func execStreamingRemote(_ prompt: String) async throws -> ZagRemoteSession {
        guard let conn = _connection else {
            throw ZagError(message: "Remote streaming requires a connection. Use .connection() or .remote(url:token:) first.")
        }
        let client = ZagRemoteClient(connection: conn, session: _urlSession ?? .shared)
        let params = buildSpawnParams(prompt: prompt)
        let spawned = try await client.spawn(params)

        // Create WebSocket connection for event streaming
        let httpURL = conn.baseURL.appendingPathComponent("/api/v1/sessions/\(spawned.sessionId)/stream")
        var components = URLComponents(url: httpURL, resolvingAgainstBaseURL: false)!
        components.scheme = conn.baseURL.scheme == "https" ? "wss" : "ws"
        let wsURL = components.url!

        var request = URLRequest(url: wsURL)
        request.setValue("Bearer \(conn.token)", forHTTPHeaderField: "Authorization")

        let urlSession = _urlSession ?? .shared
        let webSocketTask = urlSession.webSocketTask(with: request)

        return ZagRemoteSession(webSocketTask: webSocketTask, client: client, sessionId: spawned.sessionId)
    }

    /// Start an interactive agent session (inherits stdio).
    /// Only available in local mode (macOS/Linux).
    #if os(macOS) || os(Linux)
    public func run(_ prompt: String? = nil) async throws {
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capabilityRequirements())
        var args: [String] = ["run"]
        args += buildGlobalArgs()
        if _json { args.append("--json") }
        if let s = _jsonSchema { args += ["--json-schema", s] }
        if let p = prompt { args.append(p) }
        try await ZagProcess.runInteractive(bin: bin, args: args)
    }

    /// Resume a previous session by ID.
    public func resume(_ sessionId: String) async throws {
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capabilityRequirements())
        var args: [String] = ["run"]
        args += buildGlobalArgs()
        args += ["--resume", sessionId]
        try await ZagProcess.runInteractive(bin: bin, args: args)
    }

    /// Resume the most recent session.
    public func continueLast() async throws {
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capabilityRequirements())
        var args: [String] = ["run"]
        args += buildGlobalArgs()
        args.append("--continue")
        try await ZagProcess.runInteractive(bin: bin, args: args)
    }

    /// Resume a previous session non-interactively with a follow-up prompt.
    public func execResume(sessionId: String, prompt: String) async throws -> AgentOutput {
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capabilityRequirements())
        var args = buildExecArgs(prompt: prompt)
        let promptIdx = args.count - 1
        args.insert(contentsOf: ["--resume", sessionId], at: promptIdx)
        return try await ZagProcess.exec(bin: bin, args: args)
    }

    /// Resume the most recent session non-interactively with a follow-up prompt.
    public func execContinue(prompt: String) async throws -> AgentOutput {
        try await VersionCheck.check(bin: bin, requirements: versionRequirements())
        try await CapabilityCheck.check(bin: bin, provider: _provider, requirements: capabilityRequirements())
        var args = buildExecArgs(prompt: prompt)
        let promptIdx = args.count - 1
        args.insert("--continue", at: promptIdx)
        return try await ZagProcess.exec(bin: bin, args: args)
    }

    /// Resume a previous session in streaming mode with a follow-up prompt.
    public func streamResume(sessionId: String, prompt: String) -> AsyncThrowingStream<Event, Error> {
        let binPath = bin
        let requirements = versionRequirements()
        let capReqs = capabilityRequirements()
        let capturedProvider = _provider
        var args = buildExecArgs(prompt: prompt, streaming: true)
        let promptIdx = args.count - 1
        args.insert(contentsOf: ["--resume", sessionId], at: promptIdx)
        let capturedArgs = args
        return AsyncThrowingStream { continuation in
            Task {
                do {
                    try await VersionCheck.check(bin: binPath, requirements: requirements)
                    try await CapabilityCheck.check(bin: binPath, provider: capturedProvider, requirements: capReqs)
                    let innerStream = ZagProcess.stream(bin: binPath, args: capturedArgs)
                    for try await event in innerStream {
                        continuation.yield(event)
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }

    /// Resume the most recent session in streaming mode with a follow-up prompt.
    public func streamContinue(prompt: String) -> AsyncThrowingStream<Event, Error> {
        let binPath = bin
        let requirements = versionRequirements()
        let capReqs = capabilityRequirements()
        let capturedProvider = _provider
        var args = buildExecArgs(prompt: prompt, streaming: true)
        let promptIdx = args.count - 1
        args.insert("--continue", at: promptIdx)
        let capturedArgs = args
        return AsyncThrowingStream { continuation in
            Task {
                do {
                    try await VersionCheck.check(bin: binPath, requirements: requirements)
                    try await CapabilityCheck.check(bin: binPath, provider: capturedProvider, requirements: capReqs)
                    let innerStream = ZagProcess.stream(bin: binPath, args: capturedArgs)
                    for try await event in innerStream {
                        continuation.yield(event)
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }
    #endif

    /// Get a configured `ZagRemoteClient` from this builder's connection settings.
    /// Useful for direct access to the full remote API.
    public func remoteClient() throws -> ZagRemoteClient {
        guard let conn = _connection else {
            throw ZagError(message: "No remote connection configured. Use .connection() or .remote(url:token:) first.")
        }
        return ZagRemoteClient(connection: conn, session: _urlSession ?? .shared)
    }
}
