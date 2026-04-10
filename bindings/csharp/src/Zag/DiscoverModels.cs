using System.Text.Json.Serialization;

namespace Zag;

/// <summary>Feature support declaration for a provider capability.</summary>
public record FeatureSupport
{
    [JsonPropertyName("supported")]
    public bool Supported { get; init; }

    [JsonPropertyName("native")]
    public bool Native { get; init; }
}

/// <summary>Session log support with completeness level.</summary>
public record SessionLogSupport
{
    [JsonPropertyName("supported")]
    public bool Supported { get; init; }

    [JsonPropertyName("native")]
    public bool Native { get; init; }

    [JsonPropertyName("completeness")]
    public string? Completeness { get; init; }
}

/// <summary>
/// Streaming input support with mid-turn injection semantics.
/// <para>
/// <c>Semantics</c> describes what happens when
/// <c>send_user_message</c> is called while the agent is producing a
/// response on the current turn. One of <c>"queue"</c>,
/// <c>"interrupt"</c>, or <c>"between-turns-only"</c>. <c>null</c> when
/// <c>Supported</c> is <c>false</c>.
/// </para>
/// </summary>
public record StreamingInputSupport
{
    [JsonPropertyName("supported")]
    public bool Supported { get; init; }

    [JsonPropertyName("native")]
    public bool Native { get; init; }

    [JsonPropertyName("semantics")]
    public string? Semantics { get; init; }
}

/// <summary>Size alias mappings (small/medium/large to actual model names).</summary>
public record SizeMappings
{
    [JsonPropertyName("small")]
    public string Small { get; init; } = "";

    [JsonPropertyName("medium")]
    public string Medium { get; init; } = "";

    [JsonPropertyName("large")]
    public string Large { get; init; } = "";
}

/// <summary>All feature flags for a provider.</summary>
public record Features
{
    [JsonPropertyName("interactive")]
    public FeatureSupport Interactive { get; init; } = new();

    [JsonPropertyName("non_interactive")]
    public FeatureSupport NonInteractive { get; init; } = new();

    [JsonPropertyName("resume")]
    public FeatureSupport Resume { get; init; } = new();

    [JsonPropertyName("resume_with_prompt")]
    public FeatureSupport ResumeWithPrompt { get; init; } = new();

    [JsonPropertyName("session_logs")]
    public SessionLogSupport SessionLogs { get; init; } = new();

    [JsonPropertyName("json_output")]
    public FeatureSupport JsonOutput { get; init; } = new();

    [JsonPropertyName("stream_json")]
    public FeatureSupport StreamJson { get; init; } = new();

    [JsonPropertyName("json_schema")]
    public FeatureSupport JsonSchema { get; init; } = new();

    [JsonPropertyName("input_format")]
    public FeatureSupport InputFormat { get; init; } = new();

    [JsonPropertyName("streaming_input")]
    public StreamingInputSupport StreamingInput { get; init; } = new();

    [JsonPropertyName("worktree")]
    public FeatureSupport Worktree { get; init; } = new();

    [JsonPropertyName("sandbox")]
    public FeatureSupport Sandbox { get; init; } = new();

    [JsonPropertyName("system_prompt")]
    public FeatureSupport SystemPrompt { get; init; } = new();

    [JsonPropertyName("auto_approve")]
    public FeatureSupport AutoApprove { get; init; } = new();

    [JsonPropertyName("review")]
    public FeatureSupport Review { get; init; } = new();

    [JsonPropertyName("add_dirs")]
    public FeatureSupport AddDirs { get; init; } = new();

    [JsonPropertyName("max_turns")]
    public FeatureSupport MaxTurns { get; init; } = new();
}

/// <summary>Full capability declaration for a provider.</summary>
public record ProviderCapability
{
    [JsonPropertyName("provider")]
    public string Provider { get; init; } = "";

    [JsonPropertyName("default_model")]
    public string DefaultModel { get; init; } = "";

    [JsonPropertyName("available_models")]
    public List<string> AvailableModels { get; init; } = [];

    [JsonPropertyName("size_mappings")]
    public SizeMappings SizeMappings { get; init; } = new();

    [JsonPropertyName("features")]
    public Features Features { get; init; } = new();
}

/// <summary>Result of resolving a model alias.</summary>
public record ResolvedModel
{
    [JsonPropertyName("input")]
    public string Input { get; init; } = "";

    [JsonPropertyName("resolved")]
    public string Resolved { get; init; } = "";

    [JsonPropertyName("is_alias")]
    public bool IsAlias { get; init; }

    [JsonPropertyName("provider")]
    public string Provider { get; init; } = "";
}
