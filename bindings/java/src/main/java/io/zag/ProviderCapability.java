package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/** Full capability declaration for a provider. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record ProviderCapability(
        @JsonProperty("provider") String provider,
        @JsonProperty("default_model") String defaultModel,
        @JsonProperty("available_models") List<String> availableModels,
        @JsonProperty("size_mappings") SizeMappings sizeMappings,
        @JsonProperty("features") Features features) {

    /** Feature support declaration for a provider capability. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    public record FeatureSupport(
            @JsonProperty("supported") boolean supported,
            @JsonProperty("native") boolean native_) {}

    /** Session log support with completeness level. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    public record SessionLogSupport(
            @JsonProperty("supported") boolean supported,
            @JsonProperty("native") boolean native_,
            @JsonProperty("completeness") String completeness) {}

    /** Size alias mappings (small/medium/large to actual model names). */
    @JsonIgnoreProperties(ignoreUnknown = true)
    public record SizeMappings(
            @JsonProperty("small") String small,
            @JsonProperty("medium") String medium,
            @JsonProperty("large") String large) {}

    /** All feature flags for a provider. */
    @JsonIgnoreProperties(ignoreUnknown = true)
    public record Features(
            @JsonProperty("interactive") FeatureSupport interactive,
            @JsonProperty("non_interactive") FeatureSupport nonInteractive,
            @JsonProperty("resume") FeatureSupport resume,
            @JsonProperty("resume_with_prompt") FeatureSupport resumeWithPrompt,
            @JsonProperty("session_logs") SessionLogSupport sessionLogs,
            @JsonProperty("json_output") FeatureSupport jsonOutput,
            @JsonProperty("stream_json") FeatureSupport streamJson,
            @JsonProperty("json_schema") FeatureSupport jsonSchema,
            @JsonProperty("input_format") FeatureSupport inputFormat,
            @JsonProperty("streaming_input") FeatureSupport streamingInput,
            @JsonProperty("worktree") FeatureSupport worktree,
            @JsonProperty("sandbox") FeatureSupport sandbox,
            @JsonProperty("system_prompt") FeatureSupport systemPrompt,
            @JsonProperty("auto_approve") FeatureSupport autoApprove,
            @JsonProperty("review") FeatureSupport review,
            @JsonProperty("add_dirs") FeatureSupport addDirs,
            @JsonProperty("max_turns") FeatureSupport maxTurns) {}
}
