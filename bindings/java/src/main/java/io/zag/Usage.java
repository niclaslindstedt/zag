package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

/** Token usage statistics. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record Usage(
        @JsonProperty("input_tokens") long inputTokens,
        @JsonProperty("output_tokens") long outputTokens,
        @JsonProperty("cache_read_tokens") Long cacheReadTokens,
        @JsonProperty("cache_creation_tokens") Long cacheCreationTokens,
        @JsonProperty("web_search_requests") Integer webSearchRequests,
        @JsonProperty("web_fetch_requests") Integer webFetchRequests) {}
