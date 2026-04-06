using System.Text.Json;
using System.Text.Json.Serialization;

namespace Zag;

/// <summary>Unified output from an agent session.</summary>
public record AgentOutput
{
    [JsonPropertyName("agent")]
    public string Agent { get; init; } = "";

    [JsonPropertyName("session_id")]
    public string SessionId { get; init; } = "";

    [JsonPropertyName("events")]
    public List<Event> Events { get; init; } = [];

    [JsonPropertyName("result")]
    public string? Result { get; init; }

    [JsonPropertyName("is_error")]
    public bool IsError { get; init; }

    [JsonPropertyName("exit_code")]
    public int? ExitCode { get; init; }

    [JsonPropertyName("error_message")]
    public string? ErrorMessage { get; init; }

    [JsonPropertyName("total_cost_usd")]
    public double? TotalCostUsd { get; init; }

    [JsonPropertyName("usage")]
    public Usage? Usage { get; init; }
}

/// <summary>Token usage statistics.</summary>
public record Usage
{
    [JsonPropertyName("input_tokens")]
    public long InputTokens { get; init; }

    [JsonPropertyName("output_tokens")]
    public long OutputTokens { get; init; }

    [JsonPropertyName("cache_read_tokens")]
    public long? CacheReadTokens { get; init; }

    [JsonPropertyName("cache_creation_tokens")]
    public long? CacheCreationTokens { get; init; }

    [JsonPropertyName("web_search_requests")]
    public int? WebSearchRequests { get; init; }

    [JsonPropertyName("web_fetch_requests")]
    public int? WebFetchRequests { get; init; }
}

/// <summary>Result from a tool execution.</summary>
public record ToolResult
{
    [JsonPropertyName("success")]
    public bool Success { get; init; }

    [JsonPropertyName("output")]
    public string? Output { get; init; }

    [JsonPropertyName("error")]
    public string? Error { get; init; }

    [JsonPropertyName("data")]
    public JsonElement? Data { get; init; }
}

// ---------------------------------------------------------------------------
// Content blocks
// ---------------------------------------------------------------------------

/// <summary>A block of content in an assistant message.</summary>
[JsonConverter(typeof(ContentBlockConverter))]
public abstract record ContentBlock
{
    [JsonPropertyName("type")]
    public abstract string Type { get; }
}

/// <summary>Plain text content.</summary>
public record TextBlock : ContentBlock
{
    public override string Type => "text";

    [JsonPropertyName("text")]
    public string Text { get; init; } = "";
}

/// <summary>Tool invocation content.</summary>
public record ToolUseBlock : ContentBlock
{
    public override string Type => "tool_use";

    [JsonPropertyName("id")]
    public string Id { get; init; } = "";

    [JsonPropertyName("name")]
    public string Name { get; init; } = "";

    [JsonPropertyName("input")]
    public JsonElement? Input { get; init; }
}

// ---------------------------------------------------------------------------
// Events (tagged union on "type")
// ---------------------------------------------------------------------------

/// <summary>Base class for all agent session events.</summary>
[JsonConverter(typeof(EventConverter))]
public abstract record Event
{
    [JsonPropertyName("type")]
    public abstract string Type { get; }
}

/// <summary>Session initialization event.</summary>
public record InitEvent : Event
{
    public override string Type => "init";

    [JsonPropertyName("model")]
    public string Model { get; init; } = "";

    [JsonPropertyName("tools")]
    public List<string> Tools { get; init; } = [];

    [JsonPropertyName("working_directory")]
    public string? WorkingDirectory { get; init; }

    [JsonPropertyName("metadata")]
    public Dictionary<string, JsonElement> Metadata { get; init; } = [];
}

/// <summary>User message (replayed via --replay-user-messages).</summary>
public record UserMessageEvent : Event
{
    public override string Type => "user_message";

    [JsonPropertyName("content")]
    public List<ContentBlock> Content { get; init; } = [];
}

/// <summary>Message from the assistant.</summary>
public record AssistantMessageEvent : Event
{
    public override string Type => "assistant_message";

    [JsonPropertyName("content")]
    public List<ContentBlock> Content { get; init; } = [];

    [JsonPropertyName("usage")]
    public Usage? Usage { get; init; }
}

/// <summary>Tool execution event.</summary>
public record ToolExecutionEvent : Event
{
    public override string Type => "tool_execution";

    [JsonPropertyName("tool_name")]
    public string ToolName { get; init; } = "";

    [JsonPropertyName("tool_id")]
    public string ToolId { get; init; } = "";

    [JsonPropertyName("input")]
    public JsonElement? Input { get; init; }

    [JsonPropertyName("result")]
    public ToolResult Result { get; init; } = new();
}

/// <summary>Final session result event.</summary>
public record ResultEvent : Event
{
    public override string Type => "result";

    [JsonPropertyName("success")]
    public bool Success { get; init; }

    [JsonPropertyName("message")]
    public string? Message { get; init; }

    [JsonPropertyName("duration_ms")]
    public long? DurationMs { get; init; }

    [JsonPropertyName("num_turns")]
    public int? NumTurns { get; init; }
}

/// <summary>Error event.</summary>
public record ErrorEvent : Event
{
    public override string Type => "error";

    [JsonPropertyName("message")]
    public string Message { get; init; } = "";

    [JsonPropertyName("details")]
    public JsonElement? Details { get; init; }
}

/// <summary>Permission request event.</summary>
public record PermissionRequestEvent : Event
{
    public override string Type => "permission_request";

    [JsonPropertyName("tool_name")]
    public string ToolName { get; init; } = "";

    [JsonPropertyName("description")]
    public string Description { get; init; } = "";

    [JsonPropertyName("granted")]
    public bool Granted { get; init; }
}

// ---------------------------------------------------------------------------
// Custom JSON converters for tagged unions
// ---------------------------------------------------------------------------

/// <summary>Deserializes Event based on the "type" discriminator.</summary>
public class EventConverter : JsonConverter<Event>
{
    public override Event Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        var type = root.GetProperty("type").GetString();

        var raw = root.GetRawText();
        return type switch
        {
            "init" => JsonSerializer.Deserialize<InitEvent>(raw, ConverterFreeOptions)!,
            "user_message" => JsonSerializer.Deserialize<UserMessageEvent>(raw, ConverterFreeOptions)!,
            "assistant_message" => JsonSerializer.Deserialize<AssistantMessageEvent>(raw, ConverterFreeOptions)!,
            "tool_execution" => JsonSerializer.Deserialize<ToolExecutionEvent>(raw, ConverterFreeOptions)!,
            "result" => JsonSerializer.Deserialize<ResultEvent>(raw, ConverterFreeOptions)!,
            "error" => JsonSerializer.Deserialize<ErrorEvent>(raw, ConverterFreeOptions)!,
            "permission_request" => JsonSerializer.Deserialize<PermissionRequestEvent>(raw, ConverterFreeOptions)!,
            _ => throw new JsonException($"Unknown event type: {type}")
        };
    }

    public override void Write(Utf8JsonWriter writer, Event value, JsonSerializerOptions options)
    {
        JsonSerializer.Serialize(writer, value, value.GetType(), options);
    }

    /// <summary>Options without the EventConverter to prevent infinite recursion.</summary>
    private static readonly JsonSerializerOptions ConverterFreeOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };
}

/// <summary>Deserializes ContentBlock based on the "type" discriminator.</summary>
public class ContentBlockConverter : JsonConverter<ContentBlock>
{
    public override ContentBlock Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        var type = root.GetProperty("type").GetString();

        var raw = root.GetRawText();
        return type switch
        {
            "text" => JsonSerializer.Deserialize<TextBlock>(raw, ConverterFreeOptions)!,
            "tool_use" => JsonSerializer.Deserialize<ToolUseBlock>(raw, ConverterFreeOptions)!,
            _ => throw new JsonException($"Unknown content block type: {type}")
        };
    }

    public override void Write(Utf8JsonWriter writer, ContentBlock value, JsonSerializerOptions options)
    {
        JsonSerializer.Serialize(writer, value, value.GetType(), options);
    }

    private static readonly JsonSerializerOptions ConverterFreeOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };
}

// ---------------------------------------------------------------------------
// Exception
// ---------------------------------------------------------------------------

/// <summary>Exception thrown when the zag process fails.</summary>
public class ZagException : Exception
{
    public int? ExitCode { get; }
    public string Stderr { get; }

    public ZagException(string message, int? exitCode, string stderr)
        : base(message)
    {
        ExitCode = exitCode;
        Stderr = stderr;
    }
}
