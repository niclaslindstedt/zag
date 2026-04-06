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
    private readonly List<string> _addDirs = [];
    private readonly List<string> _envVars = [];
    private bool _json;
    private object? _jsonSchema;
    private bool _jsonStream;
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

    /// <summary>Add an additional directory for the agent to include.</summary>
    public ZagBuilder AddDir(string d) { _addDirs.Add(d); return this; }

    /// <summary>Add an environment variable for the agent subprocess.</summary>
    public ZagBuilder Env(string key, string value) { _envVars.Add($"{key}={value}"); return this; }

    /// <summary>Request JSON output from the agent.</summary>
    public ZagBuilder Json() { _json = true; return this; }

    /// <summary>Set a JSON schema for structured output validation. Implies Json().</summary>
    public ZagBuilder JsonSchema(object s) { _jsonSchema = s; _json = true; return this; }

    /// <summary>Enable streaming JSON output (NDJSON format).</summary>
    public ZagBuilder JsonStream() { _jsonStream = true; return this; }

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

    /// <summary>Set the input format (Claude only, e.g., "text", "stream-json").</summary>
    public ZagBuilder InputFormat(string f) { _inputFormat = f; return this; }

    /// <summary>Re-emit user messages from stdin on stdout (Claude only).</summary>
    public ZagBuilder ReplayUserMessages(bool r = true) { _replayUserMessages = r; return this; }

    /// <summary>Include partial message chunks in streaming output (Claude only).</summary>
    public ZagBuilder IncludePartialMessages(bool i = true) { _includePartialMessages = i; return this; }

    /// <summary>Set the maximum number of agentic turns.</summary>
    public ZagBuilder MaxTurns(int n) { _maxTurns = n; return this; }

    /// <summary>Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded.</summary>
    public ZagBuilder Timeout(string t) { _timeout = t; return this; }

    /// <summary>Set MCP server config for this invocation: JSON string or file path (Claude only).</summary>
    public ZagBuilder McpConfig(string c) { _mcpConfig = c; return this; }

    /// <summary>Show token usage statistics (only applies to JSON output mode).</summary>
    public ZagBuilder ShowUsage(bool s = true) { _showUsage = s; return this; }

    /// <summary>Set the Ollama model parameter size (e.g., "2b", "9b", "35b").</summary>
    public ZagBuilder Size(string s) { _size = s; return this; }

    private IReadOnlyList<VersionCheck.Requirement> VersionRequirements() => new[]
    {
        new VersionCheck.Requirement("Env()", "0.6.0", _envVars.Count > 0),
        new VersionCheck.Requirement("McpConfig()", "0.6.0", _mcpConfig != null),
    };

    // -- Arg building --------------------------------------------------------

    internal List<string> BuildGlobalArgs()
    {
        var args = new List<string>();
        if (_provider != null) { args.Add("-p"); args.Add(_provider); }
        if (_model != null) { args.Add("--model"); args.Add(_model); }
        if (_systemPrompt != null) { args.Add("--system-prompt"); args.Add(_systemPrompt); }
        if (_root != null) { args.Add("--root"); args.Add(_root); }
        if (_autoApprove) args.Add("--auto-approve");
        foreach (var d in _addDirs) { args.Add("--add-dir"); args.Add(d); }
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
        var args = BuildGlobalArgs();
        args.Add("exec");
        if (_json) args.Add("--json");
        if (_jsonSchema != null)
        {
            args.Add("--json-schema");
            args.Add(JsonSerializer.Serialize(_jsonSchema));
        }
        if (_jsonStream || streaming) args.Add("--json-stream");
        if (_outputFormat != null) { args.Add("-o"); args.Add(_outputFormat); }
        if (_inputFormat != null) { args.Add("-i"); args.Add(_inputFormat); }
        if (_replayUserMessages) args.Add("--replay-user-messages");
        if (_includePartialMessages) args.Add("--include-partial-messages");
        if (_timeout != null) { args.Add("--timeout"); args.Add(_timeout); }
        // Default to json output for structured parsing
        if (!streaming && _outputFormat == null && !_jsonStream)
        {
            args.Add("-o");
            args.Add("json");
        }
        args.Add(prompt);
        return args;
    }

    // -- Terminal methods ----------------------------------------------------

    /// <summary>Run the agent non-interactively and return structured output.</summary>
    public async Task<AgentOutput> ExecAsync(string prompt, CancellationToken ct = default)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements(), ct);
        var args = BuildExecArgs(prompt);
        return await ZagProcess.ExecAsync(_bin, [.. args], ct);
    }

    /// <summary>Run the agent in streaming mode, yielding events as they arrive.</summary>
    public async IAsyncEnumerable<Event> StreamAsync(string prompt, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements(), ct);
        var args = BuildExecArgs(prompt, streaming: true);
        await foreach (var evt in ZagProcess.StreamAsync(_bin, [.. args], ct).WithCancellation(ct))
        {
            yield return evt;
        }
    }

    /// <summary>Run the agent with streaming input and output (Claude only).</summary>
    public async Task<StreamingSession> ExecStreaming(string prompt)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements());
        var args = BuildGlobalArgs();
        args.Add("exec");
        args.Add("-i"); args.Add("stream-json");
        args.Add("-o"); args.Add("stream-json");
        args.Add("--replay-user-messages");
        if (_includePartialMessages) args.Add("--include-partial-messages");
        args.Add(prompt);
        return ZagProcess.StartStreamingProcess(_bin, [.. args]);
    }

    /// <summary>Start an interactive agent session (inherits stdio).</summary>
    public async Task RunAsync(string? prompt = null, CancellationToken ct = default)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements(), ct);
        var args = BuildGlobalArgs();
        args.Add("run");
        if (_json) args.Add("--json");
        if (_jsonSchema != null)
        {
            args.Add("--json-schema");
            args.Add(JsonSerializer.Serialize(_jsonSchema));
        }
        if (prompt != null) args.Add(prompt);
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume a previous session by ID.</summary>
    public async Task ResumeAsync(string sessionId, CancellationToken ct = default)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements(), ct);
        var args = BuildGlobalArgs();
        args.Add("run");
        args.Add("--resume");
        args.Add(sessionId);
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume the most recent session.</summary>
    public async Task ContinueLastAsync(CancellationToken ct = default)
    {
        await VersionCheck.CheckAsync(_bin, VersionRequirements(), ct);
        var args = BuildGlobalArgs();
        args.Add("run");
        args.Add("--continue");
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }
}
