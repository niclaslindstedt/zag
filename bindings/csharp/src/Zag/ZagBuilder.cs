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
        if (_worktree is true) args.Add("-w");
        else if (_worktree is string wt) { args.Add("-w"); args.Add(wt); }
        if (_sandbox is true) args.Add("--sandbox");
        else if (_sandbox is string sb) { args.Add("--sandbox"); args.Add(sb); }
        if (_verbose) args.Add("--verbose");
        if (_quiet) args.Add("--quiet");
        if (_debug) args.Add("--debug");
        if (_sessionId != null) { args.Add("--session"); args.Add(_sessionId); }
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
        var args = BuildExecArgs(prompt);
        return await ZagProcess.ExecAsync(_bin, [.. args], ct);
    }

    /// <summary>Run the agent in streaming mode, yielding events as they arrive.</summary>
    public IAsyncEnumerable<Event> StreamAsync(string prompt, CancellationToken ct = default)
    {
        var args = BuildExecArgs(prompt, streaming: true);
        return ZagProcess.StreamAsync(_bin, [.. args], ct);
    }

    /// <summary>Start an interactive agent session (inherits stdio).</summary>
    public async Task RunAsync(string? prompt = null, CancellationToken ct = default)
    {
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
        var args = BuildGlobalArgs();
        args.Add("run");
        args.Add("--resume");
        args.Add(sessionId);
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }

    /// <summary>Resume the most recent session.</summary>
    public async Task ContinueLastAsync(CancellationToken ct = default)
    {
        var args = BuildGlobalArgs();
        args.Add("run");
        args.Add("--continue");
        await ZagProcess.RunAsync(_bin, [.. args], ct);
    }
}
