using System.Text.Json;

namespace Zag;

/// <summary>
/// Fluent builder for configuring and running zag agent sessions.
/// </summary>
/// <example>
/// <code>
/// var output = await new ZagBuilder()
///     .Provider("claude")
///     .Model("sonnet")
///     .AutoApprove()
///     .ExecAsync("write a hello world program");
///
/// Console.WriteLine(output.Result);
/// </code>
/// </example>
public class ZagBuilder
{
    private string _bin = ZagProcess.DefaultBin;
    private string? _provider;
    private string? _model;
    private string? _systemPrompt;
    private string? _root;
    private bool _autoApprove;
    private bool _headless;
    private readonly List<string> _addDirs = [];
    private readonly List<string> _files = [];
    private readonly List<string> _envVars = [];
    private bool _json;
    private object? _jsonSchema;
    private object? _worktree;   // true or string
    private object? _sandbox;    // true or string
    private bool _verbose;
    private bool _quiet;
    private bool _debug;
    private string? _sessionId;
    private string? _outputFormat;
    private string? _inputFormat;
    private bool _replayUserMessages;
    private bool _includePartialMessages;
    private int? _maxTurns;
    private string? _timeout;
    private string? _mcpConfig;
    private bool _showUsage;
    private string? _size;
    private object? _exit; // null = unset, true = bare --exit, string = --exit <hint>

    // -- Configuration methods -----------------------------------------------

    /// <summary>Override the zag binary path (default: ZAG_BIN env or "zag").</summary>
    public ZagBuilder Bin(string path) { _bin = path; return this; }

    /// <summary>Set the provider (e.g., "claude", "codex", "gemini", "copilot", "ollama").</summary>
    public ZagBuilder Provider(string p) { _provider = p; return this; }

    /// <summary>Set the model (e.g., "sonnet", "opus", "small", "large").</summary>
    public ZagBuilder Model(string m) { _model = m; return this; }

    /// <summary>Set a system prompt to configure agent behavior.</summary>
    public ZagBuilder SystemPrompt(string p) { _systemPrompt = p; return this; }

    /// <summary>Set the root directory for the agent to operate in.</summary>
    public ZagBuilder Root(string r) { _root = r; return this; }

    /// <summary>Enable auto-approve mode (skip permission prompts).</summary>
    public ZagBuilder AutoApprove(bool a = true) { _autoApprove = a; return this; }

    /// <summary>
    /// Run the provider's interactive TUI attached to a private pseudo-terminal
    /// so it is invisible to the operator. Pair with <see cref="AutoApprove"/>
    /// and <c>Exit</c> — otherwise the hidden run can block on permission
    /// prompts or finish without producing a result. The CLI enforces this.
    /// </summary>
    public ZagBuilder Headless(bool h = true) { _headless = h; return this; }

    /// <summary>Add an additional directory for the agent to include.</summary>
    public ZagBuilder AddDir(string d) { _addDirs.Add(d); return this; }

    /// <summary>Attach a file to the prompt (chainable).</summary>
    public ZagBuilder File(string path) { _files.Add(path); return this; }

    /// <summary>Add an environment variable for the agent subprocess.</summary>
    public ZagBuilder Env(string key, string value) { _envVars.Add($"{key}={value}"); return this; }

    /// <summary>Request JSON output from the agent.</summary>
    public ZagBuilder Json() { _json = true; return this; }

    /// <summary>Set a JSON schema for structured output validation. Implies Json().</summary>
    public ZagBuilder JsonSchema(object s) { _jsonSchema = s; _json = true; return this; }

    /// <summary>Enable worktree mode with an optional name.</summary>
    public ZagBuilder Worktree(string? name = null) { _worktree = name ?? (object)true; return this; }

    /// <summary>Enable sandbox mode with an optional name.</summary>
    public ZagBuilder Sandbox(string? name = null) { _sandbox = name ?? (object)true; return this; }

    /// <summary>Enable verbose output.</summary>
    public ZagBuilder Verbose(bool v = true) { _verbose = v; return this; }

    /// <summary>Enable quiet mode.</summary>
    public ZagBuilder Quiet(bool q = true) { _quiet = q; return this; }

    /// <summary>Enable debug logging.</summary>
    public ZagBuilder Debug(bool d = true) { _debug = d; return this; }

    /// <summary>Pre-set a session ID (UUID).</summary>
    public ZagBuilder SessionId(string id) { _sessionId = id; return this; }

    /// <summary>Set the output format (e.g., "text", "json", "json-pretty", "stream-json").</summary>
    public ZagBuilder OutputFormat(string f) { _outputFormat = f; return this; }

    /// <summary>
    /// Set the input format (Claude only, e.g., "text", "stream-json").
    /// No-op for Codex, Gemini, Copilot, and Ollama. See <c>docs/providers.md</c>
    /// for the full per-provider flag support matrix.
    /// </summary>
    public ZagBuilder InputFormat(string f) { _inputFormat = f; return this; }

    /// <summary>
    /// Re-emit user messages from stdin on stdout (Claude only).
    /// Requires <c>-i stream-json</c> and <c>-o stream-json</c>.
    /// <see cref="ExecStreaming"/> auto-enables this flag, so most callers
    /// never need to set it manually. No-op for non-Claude providers.
    /// </summary>
    public ZagBuilder ReplayUserMessages(bool r = true) { _replayUserMessages = r; return this; }

    /// <summary>
    /// Include partial message chunks in streaming output (Claude only).
    /// Defaults to <c>false</c>. When <c>false</c>, streaming emits one
    /// <c>assistant_message</c> event per complete assistant turn. When
    /// <c>true</c>, the agent instead emits token-level partial
    /// <c>assistant_message</c> chunks as the model generates them — use
    /// this with <see cref="ExecStreaming"/> for responsive, token-by-token
    /// UIs. No-op for non-Claude providers.
    /// </summary>
    public ZagBuilder IncludePartialMessages(bool i = true) { _includePartialMessages = i; return this; }

    /// <summary>Set the maximum number of agentic turns.</summary>
    public ZagBuilder MaxTurns(int n) { _maxTurns = n; return this; }

    /// <summary>Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded.</summary>
    public ZagBuilder Timeout(string t) { _timeout = t; return this; }

    /// <summary>
    /// Set MCP server config for this invocation: JSON string or file path (Claude only).
    /// No-op for Codex, Gemini, Copilot, and Ollama — those providers manage
    /// MCP configuration through their own CLIs or do not support it.
    /// </summary>
    public ZagBuilder McpConfig(string c) { _mcpConfig = c; return this; }

    /// <summary>Show token usage statistics (only applies to JSON output mode).</summary>
    public ZagBuilder ShowUsage(bool s = true) { _showUsage = s; return this; }

    /// <summary>Set the Ollama model parameter size (e.g., "2b", "9b", "35b").</summary>
    public ZagBuilder Size(string s) { _size = s; return this; }

    /// <summary>
    /// Capture the final result via <c>zag ps kill self &lt;result&gt;</c>
    /// instead of running in <c>exec</c> mode. Only meaningful with
    /// <see cref="RunAsync"/>. The optional <paramref name="hint"/> is a
    /// short description of the expected result; when set, the kill
    /// command rejects empty results.
    /// </summary>
    public ZagBuilder Exit(string? hint = null) { _exit = hint ?? (object)true; return this; }

    private IReadOnlyList<VersionCheck.Requirement> VersionRequirements() => new[]
    {
        new VersionCheck.Requirement("Env()", "0.6.0", _envVars.Count > 0),
        new VersionCheck.Requirement("McpConfig()", "0.6.0", _mcpConfig != null),
    };

    private IReadOnlyList<CapabilityCheck.Requirement> FeatureRequirements(
        IEnumerable<CapabilityCheck.Requirement>? extras = null)
    {
        var reqs = new List<CapabilityCheck.Requirement>
        {
            new("Worktree()", CapabilityCheck.FeatureKeys.Worktree, _worktree != null),
            new("Sandbox()", CapabilityCheck.FeatureKeys.Sandbox, _sandbox != null),
            new("SystemPrompt()", CapabilityCheck.FeatureKeys.SystemPrompt, _systemPrompt != null),
            new("AddDir()", CapabilityCheck.FeatureKeys.AddDirs, _addDirs.Count > 0),
            new("MaxTurns()", CapabilityCheck.FeatureKeys.MaxTurns, _maxTurns.HasValue),
        };
        if (extras != null) reqs.AddRange(extras);
        return reqs;
    }

    /// <summary>Run version + capability preflight checks before spawning.</summary>
    private async Task PreflightAsync(
        IEnumerable<CapabilityCheck.Requirement>? extras = null,
        CancellationToken ct = default)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements(), ct);
        await CapabilityCheck.CheckAsync(_bin, _provider, FeatureRequirements(extras), ct);
    }

    // -- Arg building --------------------------------------------------------

    internal List<string> BuildGlobalArgs()
    {
        var args = new List<string>();
        if (_provider != null) { args.Add("-p"); args.Add(_provider); }
        if (_model != null) { args.Add("--model"); args.Add(_model); }
        if (_systemPrompt != null) { args.Add("--system-prompt"); args.Add(_systemPrompt); }
        if (_root != null) { args.Add("--root"); args.Add(_root); }
        if (_autoApprove) args.Add("--auto-approve");
        if (_headless) args.Add("--headless");
        foreach (var d in _addDirs) { args.Add("--add-dir"); args.Add(d); }
        foreach (var f in _files) { args.Add("--file"); args.Add(f); }
        foreach (var e in _envVars) { args.Add("--env"); args.Add(e); }
        if (_worktree is true) args.Add("-w");
        else if (_worktree is string wt) { args.Add("-w"); args.Add(wt); }
        if (_sandbox is true) args.Add("--sandbox");
        else if (_sandbox is string sb) { args.Add("--sandbox"); args.Add(sb); }
        if (_verbose) args.Add("--verbose");
        if (_quiet) args.Add("--quiet");
        if (_debug) args.Add("--debug");
        if (_sessionId != null) { args.Add("--session"); args.Add(_sessionId); }
        if (_maxTurns.HasValue) { args.Add("--max-turns"); args.Add(_maxTurns.Value.ToString()); }
        if (_mcpConfig != null) { args.Add("--mcp-config"); args.Add(_mcpConfig); }
        if (_showUsage) args.Add("--show-usage");
        if (_size != null) { args.Add("--size"); args.Add(_size); }
        return args;
    }

    internal List<string> BuildExecArgs(string prompt, bool streaming = false)
    {
        var args = new List<string> { "exec" };
        args.AddRange(BuildGlobalArgs());
        if (_json) args.Add("--json");
        if (_jsonSchema != null)
        {
            args.Add("--json-schema");
            args.Add(JsonSerializer.Serialize(_jsonSchema));
        }
        if (_outputFormat != null) { args.Add("-o"); args.Add(_outputFormat); }
        else if (streaming) { args.Add("-o"); args.Add("stream-json"); }
        // Default to json output for structured parsing
        else { args.Add("-o"); args.Add("json"); }
        if (_inputFormat != null) { args.Add("-i"); args.Add(_inputFormat); }
        if (_replayUserMessages) args.Add("--replay-user-messages");
        if (_includePartialMessages) args.Add("--include-partial-messages");
        if (_timeout != null) { args.Add("--timeout"); args.Add(_timeout); }
        args.Add("--prompt");
        args.Add(prompt);
        return args;
    }

    // -- Terminal methods ----------------------------------------------------

    /// <summary>Run the agent non-interactively and return structured output.</summary>
    public async Task<AgentOutput> ExecAsync(string prompt, CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildExecArgs(prompt);
        return await ZagProcess.ExecAsync(_bin, [.. args], ct);
    }

    /// <summary>Run the agent in streaming mode, yielding events as they arrive.</summary>
    public async IAsyncEnumerable<Event> StreamAsync(string prompt, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildExecArgs(prompt, streaming: true);
        await foreach (var evt in ZagProcess.StreamAsync(_bin, [.. args], ct).WithCancellation(ct))
        {
            yield return evt;
        }
    }

    /// <summary>
    /// Run the agent with streaming input and output (Claude only).
    /// Automatically sets <c>-i stream-json</c>, <c>-o stream-json</c>, and
    /// <c>--replay-user-messages</c>.
    ///
    /// <para><b>Default emission granularity:</b> by default
    /// <c>assistant_message</c> events are emitted once per complete
    /// assistant turn — you get one event when the model finishes speaking,
    /// not a stream of token chunks. Call
    /// <see cref="IncludePartialMessages(bool)"/> with <c>true</c> on the
    /// builder before <c>ExecStreaming</c> to receive token-level partial
    /// <c>assistant_message</c> chunks instead. The default stays
    /// <c>false</c> so existing callers that render whole-turn bubbles are
    /// not broken.</para>
    ///
    /// <para>See <c>docs/providers.md</c> for the full per-provider flag
    /// support matrix.</para>
    /// </summary>
    public async Task<StreamingSession> ExecStreaming(string prompt)
    {
        await PreflightAsync(
        [
            new CapabilityCheck.Requirement(
                "ExecStreaming()",
                CapabilityCheck.FeatureKeys.StreamingInput,
                true),
        ]);
        var args = new List<string> { "exec" };
        args.AddRange(BuildGlobalArgs());
        args.Add("-i"); args.Add("stream-json");
        args.Add("-o"); args.Add("stream-json");
        args.Add("--replay-user-messages");
        if (_includePartialMessages) args.Add("--include-partial-messages");
        args.Add("--prompt");
        args.Add(prompt);
        return ZagProcess.StartStreamingProcess(_bin, [.. args]);
    }

    /// <summary>Build CLI args for <c>run</c> interactive mode.</summary>
    internal List<string> BuildRunArgs(string? prompt = null)
    {
        var args = new List<string> { "run" };
        args.AddRange(BuildGlobalArgs());
        if (_json) args.Add("--json");
        if (_jsonSchema != null)
        {
            args.Add("--json-schema");
            args.Add(JsonSerializer.Serialize(_jsonSchema));
        }
        if (_exit is true) args.Add("--exit");
        else if (_exit is string exitHint) { args.Add("--exit"); args.Add(exitHint); }
        if (prompt != null) { args.Add("--prompt"); args.Add(prompt); }
        return args;
    }

    /// <summary>Start an interactive agent session (inherits stdio).</summary>
    public async Task RunAsync(string? prompt = null, CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildRunArgs(prompt);
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume a previous session by ID.</summary>
    public async Task ResumeAsync(string sessionId, CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = new List<string> { "run" };
        args.AddRange(BuildGlobalArgs());
        args.Add("--resume");
        args.Add(sessionId);
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume the most recent session.</summary>
    public async Task ContinueLastAsync(CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = new List<string> { "run" };
        args.AddRange(BuildGlobalArgs());
        args.Add("--continue");
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume a previous session non-interactively with a follow-up prompt.</summary>
    public async Task<AgentOutput> ExecResumeAsync(string sessionId, string prompt, CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildExecArgs(prompt);
        int idx = args.Count - 2; // insert before "--prompt", prompt
        args.Insert(idx, "--resume");
        args.Insert(idx + 1, sessionId);
        return await ZagProcess.ExecAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume the most recent session non-interactively with a follow-up prompt.</summary>
    public async Task<AgentOutput> ExecContinueAsync(string prompt, CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildExecArgs(prompt);
        args.Insert(args.Count - 2, "--continue");
        return await ZagProcess.ExecAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume a previous session in streaming mode with a follow-up prompt.</summary>
    public async IAsyncEnumerable<Event> StreamResumeAsync(string sessionId, string prompt, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildExecArgs(prompt, streaming: true);
        int idx = args.Count - 2; // insert before "--prompt", prompt
        args.Insert(idx, "--resume");
        args.Insert(idx + 1, sessionId);
        await foreach (var evt in ZagProcess.StreamAsync(_bin, [.. args], ct).WithCancellation(ct))
        {
            yield return evt;
        }
    }

    /// <summary>Resume the most recent session in streaming mode with a follow-up prompt.</summary>
    public async IAsyncEnumerable<Event> StreamContinueAsync(string prompt, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        await PreflightAsync(ct: ct);
        var args = BuildExecArgs(prompt, streaming: true);
        args.Insert(args.Count - 2, "--continue");
        await foreach (var evt in ZagProcess.StreamAsync(_bin, [.. args], ct).WithCancellation(ct))
        {
            yield return evt;
        }
    }
}
