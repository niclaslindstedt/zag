package zag

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emitAll
import kotlinx.coroutines.flow.flow
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
    private val _files: MutableList<String> = mutableListOf()
    private val _envVars: MutableList<String> = mutableListOf()
    private var _json: Boolean = false
    private var _jsonSchema: Any? = null
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
    private var _timeout: String? = null
    private var _mcpConfig: String? = null
    private var _showUsage: Boolean = false
    private var _size: String? = null
    /** `null` = unset, `true` = bare `--exit`, [String] = `--exit <hint>`. */
    private var _exit: Any? = null

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

    /** Attach a file to the prompt (chainable). */
    fun file(path: String) = apply { _files.add(path) }

    /** Add an environment variable for the agent subprocess. */
    fun env(key: String, value: String) = apply { _envVars.add("$key=$value") }

    /** Request JSON output from the agent. */
    fun json() = apply { _json = true }

    /** Set a JSON schema for structured output validation. Implies json(). */
    fun jsonSchema(s: Any) = apply { _jsonSchema = s; _json = true }

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

    /**
     * Set the input format (Claude only, e.g., "text", "stream-json").
     *
     * No-op for Codex, Gemini, Copilot, and Ollama. See `docs/providers.md`
     * for the full per-provider flag support matrix.
     */
    fun inputFormat(f: String) = apply { _inputFormat = f }

    /**
     * Re-emit user messages from stdin on stdout (Claude only).
     *
     * Requires `-i stream-json` and `-o stream-json`. [execStreaming]
     * auto-enables this flag, so most callers never need to set it
     * manually. No-op for non-Claude providers.
     */
    fun replayUserMessages(r: Boolean = true) = apply { _replayUserMessages = r }

    /**
     * Include partial message chunks in streaming output (Claude only).
     *
     * Defaults to `false`. When `false`, streaming emits one
     * `assistant_message` event per complete assistant turn. When `true`,
     * the agent instead emits token-level partial `assistant_message`
     * chunks as the model generates them — use this with [execStreaming]
     * for responsive, token-by-token UIs. No-op for non-Claude providers.
     */
    fun includePartialMessages(i: Boolean = true) = apply { _includePartialMessages = i }

    /** Set the maximum number of agentic turns. */
    fun maxTurns(n: Int) = apply { _maxTurns = n }

    /** Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. */
    fun timeout(t: String) = apply { _timeout = t }

    /**
     * Set MCP server config for this invocation: JSON string or file path (Claude only).
     *
     * No-op for Codex, Gemini, Copilot, and Ollama — those providers manage
     * MCP configuration through their own CLIs or do not support it.
     */
    fun mcpConfig(c: String) = apply { _mcpConfig = c }

    /** Show token usage statistics (only applies to JSON output mode). */
    fun showUsage(s: Boolean = true) = apply { _showUsage = s }

    /** Set the Ollama model parameter size (e.g., "2b", "9b", "35b"). */
    fun size(s: String) = apply { _size = s }

    /**
     * Capture the final result of an interactive session via
     * `zag ps kill self <result>` instead of running in `exec` mode.
     *
     * Only meaningful with [run]. The optional [hint] is a short
     * description of the expected result; when set, the kill command
     * rejects empty results.
     */
    fun exit(hint: String? = null) = apply { _exit = hint ?: true }

    // -- Arg building --------------------------------------------------------

    internal fun buildGlobalArgs(): List<String> {
        val args = mutableListOf<String>()
        _provider?.let { args.addAll(listOf("-p", it)) }
        _model?.let { args.addAll(listOf("--model", it)) }
        _systemPrompt?.let { args.addAll(listOf("--system-prompt", it)) }
        _root?.let { args.addAll(listOf("--root", it)) }
        if (_autoApprove) args.add("--auto-approve")
        for (d in _addDirs) { args.addAll(listOf("--add-dir", d)) }
        for (f in _files) { args.addAll(listOf("--file", f)) }
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
        val args = mutableListOf("exec")
        args.addAll(buildGlobalArgs())
        if (_json) args.add("--json")
        _jsonSchema?.let {
            args.add("--json-schema")
            args.add(Json.encodeToString(it.toString()))
        }
        when {
            _outputFormat != null -> args.addAll(listOf("-o", _outputFormat!!))
            streaming -> args.addAll(listOf("-o", "stream-json"))
            // Default to json output for structured parsing
            else -> args.addAll(listOf("-o", "json"))
        }
        _inputFormat?.let { args.addAll(listOf("-i", it)) }
        if (_replayUserMessages) args.add("--replay-user-messages")
        if (_includePartialMessages) args.add("--include-partial-messages")
        _timeout?.let { args.addAll(listOf("--timeout", it)) }
        args.add(prompt)
        return args
    }

    // -- Version checking ---------------------------------------------------

    private fun versionRequirements() = listOf(
        VersionCheck.Requirement("env()", "0.6.0", _envVars.isNotEmpty()),
        VersionCheck.Requirement("mcpConfig()", "0.6.0", _mcpConfig != null),
    )

    private fun featureRequirements(
        extras: List<CapabilityCheck.Requirement> = emptyList(),
    ): List<CapabilityCheck.Requirement> = listOf(
        CapabilityCheck.Requirement(
            "worktree()", CapabilityCheck.FeatureKeys.WORKTREE, _worktree != null),
        CapabilityCheck.Requirement(
            "sandbox()", CapabilityCheck.FeatureKeys.SANDBOX, _sandbox != null),
        CapabilityCheck.Requirement(
            "systemPrompt()", CapabilityCheck.FeatureKeys.SYSTEM_PROMPT, _systemPrompt != null),
        CapabilityCheck.Requirement(
            "addDir()", CapabilityCheck.FeatureKeys.ADD_DIRS, _addDirs.isNotEmpty()),
        CapabilityCheck.Requirement(
            "maxTurns()", CapabilityCheck.FeatureKeys.MAX_TURNS, _maxTurns != null),
    ) + extras

    /** Run version + capability preflight checks before spawning. */
    private suspend fun preflight(extras: List<CapabilityCheck.Requirement> = emptyList()) {
        VersionCheck.check(_bin, versionRequirements())
        CapabilityCheck.check(_bin, _provider, featureRequirements(extras))
    }

    // -- Terminal methods ----------------------------------------------------

    /** Run the agent non-interactively and return structured output. */
    suspend fun exec(prompt: String): AgentOutput {
        preflight()
        val args = buildExecArgs(prompt)
        return ZagProcess.exec(_bin, args)
    }

    /** Run the agent in streaming mode, yielding events as they arrive. */
    fun stream(prompt: String): Flow<Event> = kotlinx.coroutines.flow.flow {
        preflight()
        val args = buildExecArgs(prompt, streaming = true)
        kotlinx.coroutines.flow.emitAll(ZagProcess.stream(_bin, args))
    }

    /**
     * Run the agent with streaming input and output (Claude only).
     *
     * Automatically sets `-i stream-json`, `-o stream-json`, and
     * `--replay-user-messages`.
     *
     * ### Default emission granularity
     *
     * By default `assistant_message` events are emitted **once per complete
     * assistant turn** — you get one event when the model finishes
     * speaking, not a stream of token chunks. Call
     * [includePartialMessages] with `true` on the builder before
     * `execStreaming` to receive token-level partial `assistant_message`
     * chunks instead. The default stays `false` so existing callers that
     * render whole-turn bubbles are not broken.
     *
     * See `docs/providers.md` for the full per-provider flag support
     * matrix.
     */
    suspend fun execStreaming(prompt: String): StreamingSession {
        preflight(listOf(CapabilityCheck.Requirement(
            "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true)))
        val args = mutableListOf("exec")
        args.addAll(buildGlobalArgs())
        args.addAll(listOf("-i", "stream-json"))
        args.addAll(listOf("-o", "stream-json"))
        args.add("--replay-user-messages")
        if (_includePartialMessages) args.add("--include-partial-messages")
        args.add(prompt)
        return ZagProcess.startStreamingProcess(_bin, args)
    }

    /** Build CLI args for `run` interactive mode. */
    internal fun buildRunArgs(prompt: String? = null): List<String> {
        val args = mutableListOf("run")
        args.addAll(buildGlobalArgs())
        if (_json) args.add("--json")
        _jsonSchema?.let {
            args.add("--json-schema")
            args.add(Json.encodeToString(it.toString()))
        }
        when (val e = _exit) {
            true -> args.add("--exit")
            is String -> args.addAll(listOf("--exit", e))
        }
        prompt?.let { args.add(it) }
        return args
    }

    /** Start an interactive agent session (inherits stdio). */
    suspend fun run(prompt: String? = null) {
        preflight()
        ZagProcess.run(_bin, buildRunArgs(prompt))
    }

    /** Resume a previous session by ID. */
    suspend fun resume(sessionId: String) {
        preflight()
        val args = mutableListOf("run")
        args.addAll(buildGlobalArgs())
        args.add("--resume")
        args.add(sessionId)
        ZagProcess.run(_bin, args)
    }

    /** Resume the most recent session. */
    suspend fun continueLast() {
        preflight()
        val args = mutableListOf("run")
        args.addAll(buildGlobalArgs())
        args.add("--continue")
        ZagProcess.run(_bin, args)
    }

    /** Resume a previous session non-interactively with a follow-up prompt. */
    suspend fun execResume(sessionId: String, prompt: String): AgentOutput {
        preflight()
        val args = buildExecArgs(prompt).toMutableList()
        val promptIdx = args.size - 1
        args.add(promptIdx, "--resume")
        args.add(promptIdx + 1, sessionId)
        return ZagProcess.exec(_bin, args)
    }

    /** Resume the most recent session non-interactively with a follow-up prompt. */
    suspend fun execContinue(prompt: String): AgentOutput {
        preflight()
        val args = buildExecArgs(prompt).toMutableList()
        val promptIdx = args.size - 1
        args.add(promptIdx, "--continue")
        return ZagProcess.exec(_bin, args)
    }

    /** Resume a previous session in streaming mode with a follow-up prompt. */
    fun streamResume(sessionId: String, prompt: String): Flow<Event> = kotlinx.coroutines.flow.flow {
        preflight()
        val args = buildExecArgs(prompt, streaming = true).toMutableList()
        val promptIdx = args.size - 1
        args.add(promptIdx, "--resume")
        args.add(promptIdx + 1, sessionId)
        kotlinx.coroutines.flow.emitAll(ZagProcess.stream(_bin, args))
    }

    /** Resume the most recent session in streaming mode with a follow-up prompt. */
    fun streamContinue(prompt: String): Flow<Event> = kotlinx.coroutines.flow.flow {
        preflight()
        val args = buildExecArgs(prompt, streaming = true).toMutableList()
        val promptIdx = args.size - 1
        args.add(promptIdx, "--continue")
        kotlinx.coroutines.flow.emitAll(ZagProcess.stream(_bin, args))
    }
}
