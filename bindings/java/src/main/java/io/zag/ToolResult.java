package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.databind.JsonNode;

/** Result from a tool execution. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record ToolResult(
        @JsonProperty("success") boolean success,
        @JsonProperty("output") String output,
        @JsonProperty("error") String error,
        @JsonProperty("data") JsonNode data) {}
