package zag

import kotlinx.serialization.*
import kotlinx.serialization.json.*

/**
 * Unified output from an agent session.
 */
@Serializable
data class AgentOutput(
    val agent: String = "",
    @SerialName("session_id") val sessionId: String = "",
    val events: List<Event> = emptyList(),
    val result: String? = null,
    @SerialName("is_error") val isError: Boolean = false,
    @SerialName("exit_code") val exitCode: Int? = null,
    @SerialName("error_message") val errorMessage: String? = null,
    @SerialName("total_cost_usd") val totalCostUsd: Double? = null,
    val usage: Usage? = null,
)

/**
 * Token usage statistics.
 */
@Serializable
data class Usage(
    @SerialName("input_tokens") val inputTokens: Long = 0,
    @SerialName("output_tokens") val outputTokens: Long = 0,
    @SerialName("cache_read_tokens") val cacheReadTokens: Long? = null,
    @SerialName("cache_creation_tokens") val cacheCreationTokens: Long? = null,
    @SerialName("web_search_requests") val webSearchRequests: Int? = null,
    @SerialName("web_fetch_requests") val webFetchRequests: Int? = null,
)

/**
 * Result from a tool execution.
 */
@Serializable
data class ToolResult(
    val success: Boolean = false,
    val output: String? = null,
    val error: String? = null,
    val data: JsonElement? = null,
)

// ---------------------------------------------------------------------------
// Content blocks (tagged union on "type")
// ---------------------------------------------------------------------------

/**
 * A block of content in a message.
 */
@Serializable(with = ContentBlockSerializer::class)
sealed class ContentBlock {
    abstract val type: String
}

/**
 * Plain text content.
 */
@Serializable
data class TextBlock(
    val text: String = "",
) : ContentBlock() {
    @SerialName("type")
    override val type: String = "text"
}

/**
 * Tool invocation content.
 */
@Serializable
data class ToolUseBlock(
    val id: String = "",
    val name: String = "",
    val input: JsonElement? = null,
) : ContentBlock() {
    @SerialName("type")
    override val type: String = "tool_use"
}

// ---------------------------------------------------------------------------
// Events (tagged union on "type")
// ---------------------------------------------------------------------------

/**
 * Base class for all agent session events.
 */
@Serializable(with = EventSerializer::class)
sealed class Event {
    abstract val type: String
}

/** Session initialization event. */
@Serializable
data class InitEvent(
    val model: String = "",
    val tools: List<String> = emptyList(),
    @SerialName("working_directory") val workingDirectory: String? = null,
    val metadata: Map<String, JsonElement> = emptyMap(),
) : Event() {
    @SerialName("type")
    override val type: String = "init"
}

/** User message (replayed via --replay-user-messages). */
@Serializable
data class UserMessageEvent(
    val content: List<ContentBlock> = emptyList(),
) : Event() {
    @SerialName("type")
    override val type: String = "user_message"
}

/** Message from the assistant. */
@Serializable
data class AssistantMessageEvent(
    val content: List<ContentBlock> = emptyList(),
    val usage: Usage? = null,
) : Event() {
    @SerialName("type")
    override val type: String = "assistant_message"
}

/** Tool execution event. */
@Serializable
data class ToolExecutionEvent(
    @SerialName("tool_name") val toolName: String = "",
    @SerialName("tool_id") val toolId: String = "",
    val input: JsonElement? = null,
    val result: ToolResult = ToolResult(),
) : Event() {
    @SerialName("type")
    override val type: String = "tool_execution"
}

/** Final session result event. */
@Serializable
data class ResultEvent(
    val success: Boolean = false,
    val message: String? = null,
    @SerialName("duration_ms") val durationMs: Long? = null,
    @SerialName("num_turns") val numTurns: Int? = null,
) : Event() {
    @SerialName("type")
    override val type: String = "result"
}

/** Error event. */
@Serializable
data class ErrorEvent(
    val message: String = "",
    val details: JsonElement? = null,
) : Event() {
    @SerialName("type")
    override val type: String = "error"
}

/** Permission request event. */
@Serializable
data class PermissionRequestEvent(
    @SerialName("tool_name") val toolName: String = "",
    val description: String = "",
    val granted: Boolean = false,
) : Event() {
    @SerialName("type")
    override val type: String = "permission_request"
}

// ---------------------------------------------------------------------------
// Custom serializers for tagged unions
// ---------------------------------------------------------------------------

internal object EventSerializer : JsonContentPolymorphicSerializer<Event>(Event::class) {
    override fun selectDeserializer(element: JsonElement): DeserializationStrategy<Event> {
        return when (element.jsonObject["type"]?.jsonPrimitive?.content) {
            "init" -> InitEvent.serializer()
            "user_message" -> UserMessageEvent.serializer()
            "assistant_message" -> AssistantMessageEvent.serializer()
            "tool_execution" -> ToolExecutionEvent.serializer()
            "result" -> ResultEvent.serializer()
            "error" -> ErrorEvent.serializer()
            "permission_request" -> PermissionRequestEvent.serializer()
            else -> throw SerializationException(
                "Unknown event type: ${element.jsonObject["type"]}"
            )
        }
    }
}

internal object ContentBlockSerializer : JsonContentPolymorphicSerializer<ContentBlock>(ContentBlock::class) {
    override fun selectDeserializer(element: JsonElement): DeserializationStrategy<ContentBlock> {
        return when (element.jsonObject["type"]?.jsonPrimitive?.content) {
            "text" -> TextBlock.serializer()
            "tool_use" -> ToolUseBlock.serializer()
            else -> throw SerializationException(
                "Unknown content block type: ${element.jsonObject["type"]}"
            )
        }
    }
}

/** Shared Json instance configured for zag output parsing. */
internal val ZagJson = Json {
    ignoreUnknownKeys = true
    isLenient = true
}
