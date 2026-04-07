package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/** Unified output from an agent session. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record AgentOutput(
        @JsonProperty("agent") String agent,
        @JsonProperty("session_id") String sessionId,
        @JsonProperty("events") List<Event> events,
        @JsonProperty("result") String result,
        @JsonProperty("is_error") boolean isError,
        @JsonProperty("exit_code") Integer exitCode,
        @JsonProperty("error_message") String errorMessage,
        @JsonProperty("total_cost_usd") Double totalCostUsd,
        @JsonProperty("usage") Usage usage) {}
