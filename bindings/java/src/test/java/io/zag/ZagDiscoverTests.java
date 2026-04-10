package io.zag;

import static org.junit.jupiter.api.Assertions.*;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.List;
import org.junit.jupiter.api.Test;

class ZagDiscoverTests {

    private static final ObjectMapper MAPPER =
            new ObjectMapper().configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    private static final String CAPABILITY_JSON = """
            {
                "provider": "claude",
                "default_model": "sonnet",
                "available_models": ["sonnet", "opus", "haiku"],
                "size_mappings": {
                    "small": "haiku",
                    "medium": "sonnet",
                    "large": "opus"
                },
                "features": {
                    "interactive": {"supported": true, "native": true},
                    "non_interactive": {"supported": true, "native": true},
                    "resume": {"supported": true, "native": true},
                    "resume_with_prompt": {"supported": true, "native": true},
                    "session_logs": {"supported": true, "native": true, "completeness": "full"},
                    "json_output": {"supported": true, "native": true},
                    "stream_json": {"supported": true, "native": true},
                    "json_schema": {"supported": false, "native": false},
                    "input_format": {"supported": true, "native": true},
                    "streaming_input": {"supported": true, "native": true, "semantics": "queue"},
                    "worktree": {"supported": true, "native": false},
                    "sandbox": {"supported": true, "native": false},
                    "system_prompt": {"supported": true, "native": true},
                    "auto_approve": {"supported": true, "native": true},
                    "review": {"supported": true, "native": true},
                    "add_dirs": {"supported": true, "native": true},
                    "max_turns": {"supported": true, "native": true}
                }
            }
            """;

    private static final String CAPABILITIES_ARRAY_JSON = """
            [
                {
                    "provider": "claude",
                    "default_model": "sonnet",
                    "available_models": ["sonnet", "opus"],
                    "size_mappings": {"small": "haiku", "medium": "sonnet", "large": "opus"},
                    "features": {
                        "interactive": {"supported": true, "native": true},
                        "non_interactive": {"supported": true, "native": true},
                        "resume": {"supported": true, "native": true},
                        "resume_with_prompt": {"supported": true, "native": true},
                        "session_logs": {"supported": true, "native": true, "completeness": "full"},
                        "json_output": {"supported": true, "native": true},
                        "stream_json": {"supported": true, "native": true},
                        "json_schema": {"supported": false, "native": false},
                        "input_format": {"supported": true, "native": true},
                        "streaming_input": {"supported": true, "native": true, "semantics": "queue"},
                        "worktree": {"supported": true, "native": false},
                        "sandbox": {"supported": true, "native": false},
                        "system_prompt": {"supported": true, "native": true},
                        "auto_approve": {"supported": true, "native": true},
                        "review": {"supported": true, "native": true},
                        "add_dirs": {"supported": true, "native": true},
                        "max_turns": {"supported": true, "native": true}
                    }
                },
                {
                    "provider": "codex",
                    "default_model": "o3-mini",
                    "available_models": ["o3-mini"],
                    "size_mappings": {"small": "o3-mini", "medium": "o3-mini", "large": "o3-mini"},
                    "features": {
                        "interactive": {"supported": false, "native": false},
                        "non_interactive": {"supported": true, "native": true},
                        "resume": {"supported": false, "native": false},
                        "resume_with_prompt": {"supported": false, "native": false},
                        "session_logs": {"supported": false, "native": false, "completeness": null},
                        "json_output": {"supported": true, "native": true},
                        "stream_json": {"supported": true, "native": true},
                        "json_schema": {"supported": false, "native": false},
                        "input_format": {"supported": false, "native": false},
                        "streaming_input": {"supported": false, "native": false},
                        "worktree": {"supported": false, "native": false},
                        "sandbox": {"supported": false, "native": false},
                        "system_prompt": {"supported": false, "native": false},
                        "auto_approve": {"supported": false, "native": false},
                        "review": {"supported": false, "native": false},
                        "add_dirs": {"supported": false, "native": false},
                        "max_turns": {"supported": false, "native": false}
                    }
                }
            ]
            """;

    private static final String RESOLVED_MODEL_JSON = """
            {
                "input": "large",
                "resolved": "opus",
                "is_alias": true,
                "provider": "claude"
            }
            """;

    @Test
    void providerCapability_deserializes() throws Exception {
        var cap = MAPPER.readValue(CAPABILITY_JSON, ProviderCapability.class);
        assertNotNull(cap);
        assertEquals("claude", cap.provider());
        assertEquals("sonnet", cap.defaultModel());
        assertEquals(List.of("sonnet", "opus", "haiku"), cap.availableModels());
    }

    @Test
    void sizeMappings_deserializes() throws Exception {
        var cap = MAPPER.readValue(CAPABILITY_JSON, ProviderCapability.class);
        var sizes = cap.sizeMappings();
        assertNotNull(sizes);
        assertEquals("haiku", sizes.small());
        assertEquals("sonnet", sizes.medium());
        assertEquals("opus", sizes.large());
    }

    @Test
    void features_deserializes() throws Exception {
        var cap = MAPPER.readValue(CAPABILITY_JSON, ProviderCapability.class);
        var features = cap.features();
        assertNotNull(features);

        assertTrue(features.interactive().supported());
        assertTrue(features.interactive().native_());
        assertTrue(features.nonInteractive().supported());
        assertTrue(features.resume().supported());
        assertTrue(features.resumeWithPrompt().supported());
        assertFalse(features.jsonSchema().supported());
        assertFalse(features.jsonSchema().native_());
        assertTrue(features.worktree().supported());
        assertFalse(features.worktree().native_());
        assertTrue(features.sandbox().supported());
        assertTrue(features.systemPrompt().supported());
        assertTrue(features.autoApprove().supported());
        assertTrue(features.review().supported());
        assertTrue(features.addDirs().supported());
        assertTrue(features.maxTurns().supported());
    }

    @Test
    void streamingInput_deserializesWithSemantics() throws Exception {
        var cap = MAPPER.readValue(CAPABILITY_JSON, ProviderCapability.class);
        var si = cap.features().streamingInput();
        assertNotNull(si);
        assertTrue(si.supported());
        assertTrue(si.native_());
        assertEquals("queue", si.semantics());
    }

    @Test
    void streamingInput_nullSemanticsWhenUnsupported() throws Exception {
        var caps = MAPPER.readValue(CAPABILITIES_ARRAY_JSON, new TypeReference<List<ProviderCapability>>() {});
        var codex = caps.get(1);
        var si = codex.features().streamingInput();
        assertFalse(si.supported());
        assertNull(si.semantics());
    }

    @Test
    void sessionLogs_deserializesWithCompleteness() throws Exception {
        var cap = MAPPER.readValue(CAPABILITY_JSON, ProviderCapability.class);
        var logs = cap.features().sessionLogs();
        assertNotNull(logs);
        assertTrue(logs.supported());
        assertTrue(logs.native_());
        assertEquals("full", logs.completeness());
    }

    @Test
    void sessionLogs_nullCompleteness() throws Exception {
        var caps = MAPPER.readValue(CAPABILITIES_ARRAY_JSON, new TypeReference<List<ProviderCapability>>() {});
        var codex = caps.get(1);
        assertNull(codex.features().sessionLogs().completeness());
    }

    @Test
    void capabilitiesArray_deserializes() throws Exception {
        var caps = MAPPER.readValue(CAPABILITIES_ARRAY_JSON, new TypeReference<List<ProviderCapability>>() {});
        assertNotNull(caps);
        assertEquals(2, caps.size());
        assertEquals("claude", caps.get(0).provider());
        assertEquals("codex", caps.get(1).provider());
    }

    @Test
    void resolvedModel_deserializes() throws Exception {
        var model = MAPPER.readValue(RESOLVED_MODEL_JSON, ResolvedModel.class);
        assertNotNull(model);
        assertEquals("large", model.input());
        assertEquals("opus", model.resolved());
        assertTrue(model.isAlias());
        assertEquals("claude", model.provider());
    }

    @Test
    void resolvedModel_nonAlias() throws Exception {
        var json = """
                {"input": "sonnet", "resolved": "sonnet", "is_alias": false, "provider": "claude"}
                """;
        var model = MAPPER.readValue(json, ResolvedModel.class);
        assertEquals("sonnet", model.input());
        assertEquals("sonnet", model.resolved());
        assertFalse(model.isAlias());
    }

    @Test
    void providerCapability_ignoresUnknownFields() throws Exception {
        var json = """
                {
                    "provider": "test",
                    "default_model": "m1",
                    "available_models": [],
                    "size_mappings": {"small": "s", "medium": "m", "large": "l"},
                    "features": {
                        "interactive": {"supported": true, "native": false},
                        "non_interactive": {"supported": true, "native": false},
                        "resume": {"supported": false, "native": false},
                        "resume_with_prompt": {"supported": false, "native": false},
                        "session_logs": {"supported": false, "native": false, "completeness": null},
                        "json_output": {"supported": true, "native": true},
                        "stream_json": {"supported": false, "native": false},
                        "json_schema": {"supported": false, "native": false},
                        "input_format": {"supported": false, "native": false},
                        "streaming_input": {"supported": false, "native": false},
                        "worktree": {"supported": false, "native": false},
                        "sandbox": {"supported": false, "native": false},
                        "system_prompt": {"supported": false, "native": false},
                        "auto_approve": {"supported": false, "native": false},
                        "review": {"supported": false, "native": false},
                        "add_dirs": {"supported": false, "native": false},
                        "max_turns": {"supported": false, "native": false},
                        "future_feature": {"supported": true, "native": true}
                    },
                    "unknown_field": "ignored"
                }
                """;
        var cap = MAPPER.readValue(json, ProviderCapability.class);
        assertEquals("test", cap.provider());
    }

    @Test
    void listProviders_extractsNames() throws Exception {
        var caps = MAPPER.readValue(CAPABILITIES_ARRAY_JSON, new TypeReference<List<ProviderCapability>>() {});
        List<String> names = caps.stream().map(ProviderCapability::provider).toList();
        assertEquals(List.of("claude", "codex"), names);
    }

    @Test
    void featureSupport_nativeKeyword() throws Exception {
        var json = """
                {"supported": true, "native": false}
                """;
        var fs = MAPPER.readValue(json, ProviderCapability.FeatureSupport.class);
        assertTrue(fs.supported());
        assertFalse(fs.native_());
    }

    @Test
    void defaultBin_usedWhenNoBinSpecified() {
        // Verify the default bin method is accessible
        String bin = ZagProcess.defaultBin();
        assertNotNull(bin);
    }
}
