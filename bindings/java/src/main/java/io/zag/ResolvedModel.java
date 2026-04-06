package io.zag;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

/** Result of resolving a model alias. */
@JsonIgnoreProperties(ignoreUnknown = true)
public record ResolvedModel(
        @JsonProperty("input") String input,
        @JsonProperty("resolved") String resolved,
        @JsonProperty("is_alias") boolean isAlias,
        @JsonProperty("provider") String provider) {}
