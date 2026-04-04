package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;
import com.fasterxml.jackson.databind.JsonNode;
import java.util.List;
import java.util.Map;

/** Base type for all agent session events. */
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, property = "type")
@JsonSubTypes({
    @JsonSubTypes.Type(value = Event.Init.class, name = "init"),
    @JsonSubTypes.Type(value = Event.UserMessage.class, name = "user_message"),
    @JsonSubTypes.Type(value = Event.AssistantMessage.class, name = "assistant_message"),
    @JsonSubTypes.Type(value = Event.ToolExecution.class, name = "tool_execution"),
    @JsonSubTypes.Type(value = Event.Result.class, name = "result"),
    @JsonSubTypes.Type(value = Event.Error.class, name = "error"),
    @JsonSubTypes.Type(value = Event.PermissionRequest.class, name = "permission_request"),
})
@JsonIgnoreProperties(ignoreUnknown = true)
public sealed interface Event {

    String type();

    /** Session initialization event. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record Init(
            @JsonProperty("model") String model,
            @JsonProperty("tools") List<String> tools,
            @JsonProperty("working_directory") String workingDirectory,
            @JsonProperty("metadata") Map<String, JsonNode> metadata)
            implements Event {
        @Override
        public String type() {
            return "init";
        }
    }

    /** User message (replayed via --replay-user-messages). */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record UserMessage(@JsonProperty("content") List<ContentBlock> content) implements Event {
        @Override
        public String type() {
            return "user_message";
        }
    }

    /** Message from the assistant. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record AssistantMessage(
            @JsonProperty("content") List<ContentBlock> content,
            @JsonProperty("usage") Usage usage)
            implements Event {
        @Override
        public String type() {
            return "assistant_message";
        }
    }

    /** Tool execution event. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record ToolExecution(
            @JsonProperty("tool_name") String toolName,
            @JsonProperty("tool_id") String toolId,
            @JsonProperty("input") JsonNode input,
            @JsonProperty("result") ToolResult result)
            implements Event {
        @Override
        public String type() {
            return "tool_execution";
        }
    }

    /** Final session result event. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record Result(
            @JsonProperty("success") boolean success,
            @JsonProperty("message") String message,
            @JsonProperty("duration_ms") Long durationMs,
            @JsonProperty("num_turns") Integer numTurns)
            implements Event {
        @Override
        public String type() {
            return "result";
        }
    }

    /** Error event. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record Error(@JsonProperty("message") String message, @JsonProperty("details") JsonNode details)
            implements Event {
        @Override
        public String type() {
            return "error";
        }
    }

    /** Permission request event. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record PermissionRequest(
            @JsonProperty("tool_name") String toolName,
            @JsonProperty("description") String description,
            @JsonProperty("granted") boolean granted)
            implements Event {
        @Override
        public String type() {
            return "permission_request";
        }
    }
}
