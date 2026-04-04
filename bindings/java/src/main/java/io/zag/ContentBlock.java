package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;
import com.fasterxml.jackson.databind.JsonNode;

/** A block of content in an assistant message. */
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, property = "type")
@JsonSubTypes({
    @JsonSubTypes.Type(value = ContentBlock.Text.class, name = "text"),
    @JsonSubTypes.Type(value = ContentBlock.ToolUse.class, name = "tool_use"),
})
@JsonIgnoreProperties(ignoreUnknown = true)
public sealed interface ContentBlock {

    String type();

    /** Plain text content. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record Text(@JsonProperty("text") String text) implements ContentBlock {
        @Override
        public String type() {
            return "text";
        }
    }

    /** Tool invocation content. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    record ToolUse(
            @JsonProperty("id") String id,
            @JsonProperty("name") String name,
            @JsonProperty("input") JsonNode input)
            implements ContentBlock {
        @Override
        public String type() {
            return "tool_use";
        }
    }
}
