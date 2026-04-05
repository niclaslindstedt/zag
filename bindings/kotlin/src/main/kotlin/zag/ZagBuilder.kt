package zag

import kotlinx.coroutines.flow.Flow
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Fluent builder for configuring and running zag agent sessions.
 *
 * ```kotlin
 * val output = ZagBuilder()
 *     .provider("claude")
 *     .model("sonnet")
 *     .autoApprove()
 *     .exec("write a hello world program")
 *
 * println(output.result)
 * ```
 */
class ZagBuilder {
    private var _bin: String = ZagProcess.defaultBin
    private var _provider: String? = null
    private var _model: String? = null
    private var _systemPrompt: String? = null
    private var _root: String? = null
    private var _autoApprove: Boolean = false
    private val _addDirs: MutableList<String> = mutableListOf()
    private val _envVars: MutableList<String> = mutableListOf()
    private var _json: Boolean = false
    private var _jsonSchema: Any? = null
    private var _jsonStream: Boolean = false
    private var _worktree: Any? = null   // true or String
    private var _sandbox: Any? = null    // true or String
    private var _verbose: Boolean = false
    private var _quiet: Boolean = false
    private var _debug: Boolean = false
    private var _sessionId: String? = null
    private var _outputFormat: String? = null
    private var _inputFormat: String? = null
    private var _replayUserMessages: Boolean = false
    private var _includePartialMessages: Boolean = false
    private var _maxTurns: Int? = null
    private var _mcpConfig: String? = null
    private var _showUsage: Boolean = false
    private var _size: String? = null

    // -- Configuration methods -----------------------------------------------

    /** Override the zag binary path (default: ZAG_BIN env or "zag"). */
    fun bin(path: String) = apply { _bin = path }

    /** Set the provider (e.g., "claude", "codex", "gemini", "copilot", "ollama"). */
    fun provider(p: String) = apply { _provider = p }

    /** Set the model (e.g., "sonnet", "opus", "small", "large"). */
    fun model(m: String) = apply { _model = m }

    /** Set a system prompt to configure agent behavior. */
    fun systemPrompt(p: String) = apply { _systemPrompt = p }

    /** Set the root directory for the agent to operate in. */
    fun root(r: String) = apply { _root = r }

    /** Enable auto-approve mode (skip permission prompts). */
    fun autoApprove(a: Boolean = true) = apply { _autoApprove = a }

    /** Add an additional directory for the agent to include. */
    fun addDir(d: String) = apply { _addDirs.add(d) }

    /** Add an environment variable for the agent subprocess. */
    fun env(key: String, value: String) = apply { _envVars.add("$key=$value") }

    /** Request JSON output from the agent. */
    fun json() = apply { _json = true }

    /** Set a JSON schema for structured output validation. Implies json(). */
    fun jsonSchema(s: Any) = apply { _jsonSchema = s; _json = true }

    /** Enable streaming JSON output (NDJSON format). */
    fun jsonStream() = apply { _jsonStream = true }

    /** Enable worktree mode with an optional name. */
    fun worktree(name: String? = null) = apply { _worktree = name ?: true }

    /** Enable sandbox mode with an optional name. */
    fun sandbox(name: String? = null) = apply { _sandbox = name ?: true }

    /** Enable verbose output. */
    fun verbose(v: Boolean = true) = apply { _verbose = v }

    /** Enable quiet mode. */
    fun quiet(q: Boolean = true) = apply { _quiet = q }

    /** Enable debug logging. */
    fun debug(d: Boolean = true) = apply { _debug = d }

    /** Pre-set a session ID (UUID). */
    fun sessionId(id: String) = apply { _sessionId = id }

    /** Set the output format (e.g., "text", "json", "json-pretty", "stream-json"). */
    fun outputFormat(f: String) = apply { _outputFormat = f }

    /** Set the input format (Claude only, e.g., "text", "stream-json"). */
    fun inputFormat(f: String) = apply { _inputFormat = f }

    /** Re-emit user messages from stdin on stdout (Claude only). */
    fun replayUserMessages(r: Boolean = true) = apply { _replayUserMessages = r }

    /** Include partial message chunks in streaming output (Claude only). */
    fun includePartialMessages(i: Boolean = true) = apply { _includePartialMessages = i }

    /** Set the maximum number of agentic turns. */
    fun maxTurns(n: Int) = apply { _maxTurns = n }

    /** Set MCP server config for this invocation: JSON string or file path (Claude only). */
    fun mcpConfig(c: String) = apply { _mcpConfig = c }

    /** Show token usage statistics (only applies to JSON output mode). */
    fun showUsage(s: Boolean = true) = apply { _showUsage = s }

    /** Set the Ollama model parameter size (e.g., "2b", "9b", "35b"). */
    fun size(s: String) = apply { _size = s }

    // -- Arg building --------------------------------------------------------

    internal fun buildGlobalArgs(): List<String> {
        val args = mutableListOf<String>()
        _provider?.let { args.addAll(listOf("-p", it)) }
        _model?.let { args.addAll(listOf("--model", it)) }
        _systemPrompt?.let { args.addAll(listOf("--system-prompt", it)) }
        _root?.let { args.addAll(listOf("--root", it)) }
        if (_autoApprove) args.add("--auto-approve")
        for (d in _addDirs) { args.addAll(listOf("--add-dir", d)) }
        for (e in _envVars) { args.addAll(listOf("--env", e)) }
        when (val w = _worktree) {
            true -> args.add("-w")
            is String -> args.addAll(listOf("-w", w))
        }
        when (val s = _sandbox) {
            true -> args.add("--sandbox")
            is String -> args.addAll(listOf("--sandbox", s))
        }
        if (_verbose) args.add("--verbose")
        if (_quiet) args.add("--quiet")
        if (_debug) args.add("--debug")
        _sessionId?.let { args.addAll(listOf("--session", it)) }
        _maxTurns?.let { args.addAll(listOf("--max-turns", it.toString())) }
        _mcpConfig?.let { args.addAll(listOf("--mcp-config", it)) }
        if (_showUsage) args.add("--show-usage")
        _size?.let { args.addAll(listOf("--size", it)) }
        return args
    }

    internal fun buildExecArgs(prompt: String, streaming: Boolean = false): List<String> {
        val args = buildGlobalArgs().toMutableList()
        args.add("exec")
        if (_json) args.add("--json")
        _jsonSchema?.let {
            args.add("--json-schema")
            args.add(Json.encodeToString(it.toString()))
        }
        if (_jsonStream || streaming) args.add("--json-stream")
        _outputFormat?.let { args.addAll(listOf("-o", it)) }
        _inputFormat?.let { args.addAll(listOf("-i", it)) }
        if (_replayUserMessages) args.add("--replay-user-messages")
        if (_includePartialMessages) args.add("--include-partial-messages")
        // Default to json output for structured parsing
        if (!streaming && _outputFormat == null && !_jsonStream) {
            args.addAll(listOf("-o", "json"))
        }
        args.add(prompt)
        return args
    }

    // -- Terminal methods ----------------------------------------------------

    /** Run the agent non-interactively and return structured output. */
    suspend fun exec(prompt: String): AgentOutput {
        val args = buildExecArgs(prompt)
        return ZagProcess.exec(_bin, args)
    }

    /** Run the agent in streaming mode, yielding events as they arrive. */
    fun stream(prompt: String): Flow<Event> {
        val args = buildExecArgs(prompt, streaming = true)
        return ZagProcess.stream(_bin, args)
    }

    /** Run the agent with streaming input and output (Claude only). */
    fun execStreaming(prompt: String): StreamingSession {
        val args = buildGlobalArgs().toMutableList()
        args.add("exec")
        args.addAll(listOf("-i", "stream-json"))
        args.addAll(listOf("-o", "stream-json"))
        args.add("--replay-user-messages")
        if (_includePartialMessages) args.add("--include-partial-messages")
        args.add(prompt)
        return ZagProcess.startStreamingProcess(_bin, args)
    }

    /** Start an interactive agent session (inherits stdio). */
    suspend fun run(prompt: String? = null) {
        val args = buildGlobalArgs().toMutableList()
        args.add("run")
        if (_json) args.add("--json")
        _jsonSchema?.let {
            args.add("--json-schema")
            args.add(Json.encodeToString(it.toString()))
        }
        prompt?.let { args.add(it) }
        ZagProcess.run(_bin, args)
    }

    /** Resume a previous session by ID. */
    suspend fun resume(sessionId: String) {
        val args = buildGlobalArgs().toMutableList()
        args.add("run")
        args.add("--resume")
        args.add(sessionId)
        ZagProcess.run(_bin, args)
    }

    /** Resume the most recent session. */
    suspend fun continueLast() {
        val args = buildGlobalArgs().toMutableList()
        args.add("run")
        args.add("--continue")
        ZagProcess.run(_bin, args)
    }
}
