package io.zag;

import static org.junit.jupiter.api.Assertions.*;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.List;
import org.junit.jupiter.api.Test;

class ZagBuilderTests {

    @Test
    void builderDefaults_produceMinimalArgs() {
        var builder = new ZagBuilder();
        var args = builder.buildGlobalArgs();
        assertTrue(args.isEmpty());
    }

    @Test
    void builderMethodChaining_setsAllOptions() {
        var builder = new ZagBuilder()
                .provider("gemini")
                .model("large")
                .root("/project")
                .autoApprove()
                .addDir("/docs")
                .file("/tmp/data.csv")
                .verbose()
                .debug()
                .sessionId("sess-1")
                .timeout("5m");

        var args = builder.buildGlobalArgs();

        assertEquals(
                List.of(
                        "-p", "gemini",
                        "--model", "large",
                        "--root", "/project",
                        "--auto-approve",
                        "--add-dir", "/docs",
                        "--file", "/tmp/data.csv",
                        "--verbose",
                        "--debug",
                        "--session", "sess-1"),
                args);
    }

    @Test
    void builderEnvVars_produceCorrectArgs() {
        var builder = new ZagBuilder()
                .env("FOO", "bar")
                .env("BAZ", "qux");

        var args = builder.buildGlobalArgs();

        assertTrue(args.contains("--env"));
        int idx = args.indexOf("--env");
        assertEquals("FOO=bar", args.get(idx + 1));
        assertEquals("--env", args.get(idx + 2));
        assertEquals("BAZ=qux", args.get(idx + 3));
    }

    @Test
    void builderExecArgs_defaultsToJson() {
        var builder = new ZagBuilder().provider("claude");
        var args = builder.buildExecArgs("hello", false);

        assertTrue(args.contains("exec"));
        assertTrue(args.contains("-o"));
        assertTrue(args.contains("json"));
        assertTrue(args.contains("hello"));
    }

    @Test
    void builderExecArgs_streaming() {
        var builder = new ZagBuilder();
        var args = builder.buildExecArgs("hello", true);
        assertFalse(args.contains("--json-stream"));
        int oi = args.indexOf("-o");
        assertTrue(oi >= 0);
        assertEquals("stream-json", args.get(oi + 1));
    }

    @Test
    void builderWorktree_noName() {
        var builder = new ZagBuilder().worktree();
        var args = builder.buildGlobalArgs();
        assertTrue(args.contains("-w"));
        assertEquals(1, args.size());
    }

    @Test
    void builderWorktree_named() {
        var builder = new ZagBuilder().worktree("feat");
        var args = builder.buildGlobalArgs();
        assertEquals(List.of("-w", "feat"), args);
    }

    @Test
    void builderSandbox_noName() {
        var builder = new ZagBuilder().sandbox();
        var args = builder.buildGlobalArgs();
        assertTrue(args.contains("--sandbox"));
        assertEquals(1, args.size());
    }

    @Test
    void builderSandbox_named() {
        var builder = new ZagBuilder().sandbox("box1");
        var args = builder.buildGlobalArgs();
        assertEquals(List.of("--sandbox", "box1"), args);
    }

    @Test
    void builderExit_withHint() {
        var args = new ZagBuilder().exit("the answer").buildRunArgs("compute");
        int i = args.indexOf("--exit");
        assertTrue(i >= 0);
        assertEquals("the answer", args.get(i + 1));
    }

    @Test
    void builderExit_bare() {
        var args = new ZagBuilder().exit().buildRunArgs(null);
        assertTrue(args.contains("--exit"));
    }

    @Test
    void builderExit_omittedWhenNotSet() {
        var args = new ZagBuilder().buildRunArgs("hi");
        assertFalse(args.contains("--exit"));
    }

    @Test
    void builderMaxTurns() {
        var builder = new ZagBuilder().maxTurns(10);
        var args = builder.buildGlobalArgs();
        assertEquals(List.of("--max-turns", "10"), args);
    }

    @Test
    void builderMcpConfig() {
        var builder = new ZagBuilder().mcpConfig("./mcp.json");
        var args = builder.buildGlobalArgs();
        assertEquals(List.of("--mcp-config", "./mcp.json"), args);
    }

    @Test
    void builderShowUsage() {
        var builder = new ZagBuilder().showUsage();
        var args = builder.buildGlobalArgs();
        assertEquals(List.of("--show-usage"), args);
    }

    @Test
    void builderSize() {
        var builder = new ZagBuilder().size("35b");
        var args = builder.buildGlobalArgs();
        assertEquals(List.of("--size", "35b"), args);
    }

    @Test
    void timeout_includedInExecArgs() {
        var args = new ZagBuilder().timeout("5m").buildExecArgs("test", false);
        assertTrue(args.contains("--timeout"));
        assertTrue(args.contains("5m"));
    }

    @Test
    void resume_includedInExecArgs() {
        var args = new ZagBuilder().provider("claude").buildExecArgs("follow up", false);
        args.add(args.size() - 1, "--resume");
        args.add(args.size() - 1, "sess-123");
        assertTrue(args.contains("--resume"));
        assertTrue(args.contains("sess-123"));
        assertTrue(args.indexOf("--resume") < args.indexOf("follow up"));
    }

    @Test
    void continue_includedInExecArgs() {
        var args = new ZagBuilder().provider("claude").buildExecArgs("follow up", false);
        args.add(args.size() - 1, "--continue");
        assertTrue(args.contains("--continue"));
        assertTrue(args.indexOf("--continue") < args.indexOf("follow up"));
    }
}

class VersionCheckTests {

    @Test
    void parseSemver_valid() throws Exception {
        var v = VersionCheck.parseSemver("0.6.0");
        assertEquals(0, v.major());
        assertEquals(6, v.minor());
        assertEquals(0, v.patch());
    }

    @Test
    void parseSemver_invalid() {
        assertThrows(ZagException.class, () -> VersionCheck.parseSemver("invalid"));
        assertThrows(ZagException.class, () -> VersionCheck.parseSemver("1.2"));
        assertThrows(ZagException.class, () -> VersionCheck.parseSemver("a.b.c"));
    }

    @Test
    void semverComparison() throws Exception {
        var v050 = VersionCheck.parseSemver("0.5.0");
        var v060 = VersionCheck.parseSemver("0.6.0");
        var v070 = VersionCheck.parseSemver("0.7.0");

        assertTrue(v050.compareTo(v060) < 0);
        assertEquals(0, v060.compareTo(v060));
        assertTrue(v070.compareTo(v060) > 0);
    }

    @Test
    void check_noActiveRequirements_passes() throws Exception {
        VersionCheck.setVersionForTesting("zag", "0.5.0");
        try {
            VersionCheck.check("zag", List.of(
                new VersionCheck.Requirement("env()", "0.6.0", false)
            ));
        } finally {
            VersionCheck.clearVersionCache();
        }
    }

    @Test
    void check_sufficientVersion_passes() throws Exception {
        VersionCheck.setVersionForTesting("zag", "0.6.0");
        try {
            VersionCheck.check("zag", List.of(
                new VersionCheck.Requirement("env()", "0.6.0", true)
            ));
        } finally {
            VersionCheck.clearVersionCache();
        }
    }

    @Test
    void check_insufficientVersion_throws() {
        VersionCheck.setVersionForTesting("zag", "0.5.0");
        try {
            var ex = assertThrows(ZagException.class, () ->
                VersionCheck.check("zag", List.of(
                    new VersionCheck.Requirement("env()", "0.6.0", true)
                )));
            assertTrue(ex.getMessage().contains("env()"));
            assertTrue(ex.getMessage().contains("0.6.0"));
            assertTrue(ex.getMessage().contains("0.5.0"));
        } finally {
            VersionCheck.clearVersionCache();
        }
    }

    @Test
    void check_multipleFailures_reportsAll() {
        VersionCheck.setVersionForTesting("zag", "0.5.0");
        try {
            var ex = assertThrows(ZagException.class, () ->
                VersionCheck.check("zag", List.of(
                    new VersionCheck.Requirement("env()", "0.6.0", true),
                    new VersionCheck.Requirement("mcpConfig()", "0.6.0", true)
                )));
            assertTrue(ex.getMessage().contains("env()"));
            assertTrue(ex.getMessage().contains("mcpConfig()"));
        } finally {
            VersionCheck.clearVersionCache();
        }
    }
}

class CapabilityCheckTests {

    private static ProviderCapability fakeCapability(
            String provider,
            boolean streamingInput,
            boolean addDirs) {
        var yes = new ProviderCapability.FeatureSupport(true, true);
        var streaming = new ProviderCapability.StreamingInputSupport(
            streamingInput, streamingInput, streamingInput ? "queue" : null);
        var addDirsFs = new ProviderCapability.FeatureSupport(addDirs, false);
        var sessionLogs = new ProviderCapability.SessionLogSupport(true, true, "complete");
        return new ProviderCapability(
            provider,
            "default",
            List.of(),
            new ProviderCapability.SizeMappings("s", "m", "l"),
            new ProviderCapability.Features(
                yes, yes, yes, yes,
                sessionLogs,
                yes, yes, yes, yes,
                streaming,
                yes, yes, yes, yes, yes,
                addDirsFs,
                yes));
    }

    @Test
    void check_noActiveRequirements_passes() throws Exception {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("ollama", false, false)
        ));
        try {
            CapabilityCheck.check("zag", "ollama", List.of(
                new CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, false)
            ));
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }

    @Test
    void check_nullProvider_skips() throws Exception {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("ollama", false, false)
        ));
        try {
            CapabilityCheck.check("zag", null, List.of(
                new CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true)
            ));
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }

    @Test
    void check_supportedFeature_passes() throws Exception {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("claude", true, true)
        ));
        try {
            CapabilityCheck.check("zag", "claude", List.of(
                new CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true)
            ));
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }

    @Test
    void check_unsupportedFeature_throws() {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("claude", true, true),
            fakeCapability("ollama", false, false)
        ));
        try {
            var ex = assertThrows(ZagFeatureUnsupportedException.class, () ->
                CapabilityCheck.check("zag", "ollama", List.of(
                    new CapabilityCheck.Requirement(
                        "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true)
                )));
            assertEquals("ollama", ex.provider());
            assertEquals("streaming_input", ex.feature());
            assertEquals("execStreaming()", ex.method());
            assertEquals(List.of("claude"), ex.supportedProviders());
            assertTrue(ex.getMessage().contains("ollama"));
            assertTrue(ex.getMessage().contains("streaming_input"));
            assertTrue(ex.getMessage().contains("execStreaming()"));
            assertTrue(ex.getMessage().contains("Supported providers: claude"));
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }

    @Test
    void check_unknownProvider_skips() throws Exception {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("claude", true, true)
        ));
        try {
            CapabilityCheck.check("zag", "imaginary", List.of(
                new CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true)
            ));
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }

    @Test
    void check_addDirsUnsupported_throws() {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("claude", true, true),
            fakeCapability("ollama", false, false)
        ));
        try {
            var ex = assertThrows(ZagFeatureUnsupportedException.class, () ->
                CapabilityCheck.check("zag", "ollama", List.of(
                    new CapabilityCheck.Requirement(
                        "addDir()", CapabilityCheck.FeatureKeys.ADD_DIRS, true)
                )));
            assertEquals("add_dirs", ex.feature());
            assertEquals(List.of("claude"), ex.supportedProviders());
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }

    @Test
    void check_noProvidersSupport_throwsWithEmptySupportList() {
        CapabilityCheck.setCapabilitiesForTesting("zag", List.of(
            fakeCapability("ollama", false, false)
        ));
        try {
            var ex = assertThrows(ZagFeatureUnsupportedException.class, () ->
                CapabilityCheck.check("zag", "ollama", List.of(
                    new CapabilityCheck.Requirement(
                        "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true)
                )));
            assertTrue(ex.supportedProviders().isEmpty());
            assertTrue(ex.getMessage().contains("No providers currently support this feature"));
        } finally {
            CapabilityCheck.clearCapabilityCache();
        }
    }
}

class ZagExceptionTests {

    @Test
    void exception_containsFields() {
        var ex = new ZagException("test error", 1, "stderr output");
        assertEquals("test error", ex.getMessage());
        assertEquals(1, ex.exitCode());
        assertEquals("stderr output", ex.stderr());
        assertInstanceOf(Exception.class, ex);
    }
}

class ModelsTests {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private static final String SAMPLE_JSON = """
            {
                "agent": "claude",
                "session_id": "sess-123",
                "events": [
                    {
                        "type": "init",
                        "model": "sonnet",
                        "tools": ["Bash", "Read"],
                        "working_directory": "/home/user",
                        "metadata": {}
                    },
                    {
                        "type": "assistant_message",
                        "content": [{"type": "text", "text": "Hello!"}],
                        "usage": {"input_tokens": 100, "output_tokens": 50}
                    },
                    {
                        "type": "tool_execution",
                        "tool_name": "Bash",
                        "tool_id": "tool_123",
                        "input": {"command": "echo hello"},
                        "result": {"success": true, "output": "hello", "error": null, "data": null}
                    },
                    {
                        "type": "turn_complete",
                        "stop_reason": "end_turn",
                        "turn_index": 0,
                        "usage": {"input_tokens": 80, "output_tokens": 40}
                    },
                    {
                        "type": "result",
                        "success": true,
                        "message": "Done",
                        "duration_ms": 1500,
                        "num_turns": 2
                    },
                    {
                        "type": "error",
                        "message": "oops",
                        "details": null
                    },
                    {
                        "type": "permission_request",
                        "tool_name": "Bash",
                        "description": "run cmd",
                        "granted": true
                    }
                ],
                "result": "Hello!",
                "is_error": false,
                "total_cost_usd": 0.01,
                "usage": {"input_tokens": 100, "output_tokens": 50}
            }
            """;

    @Test
    void agentOutput_deserializes() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        assertNotNull(output);
        assertEquals("claude", output.agent());
        assertEquals("sess-123", output.sessionId());
        assertEquals(7, output.events().size());
        assertEquals("Hello!", output.result());
        assertFalse(output.isError());
        assertNull(output.exitCode());
        assertNull(output.errorMessage());
        assertEquals(0.01, output.totalCostUsd());
        assertNotNull(output.usage());
        assertEquals(100, output.usage().inputTokens());
    }

    @Test
    void events_deserializeCorrectTypes() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);

        assertInstanceOf(Event.Init.class, output.events().get(0));
        assertInstanceOf(Event.AssistantMessage.class, output.events().get(1));
        assertInstanceOf(Event.ToolExecution.class, output.events().get(2));
        assertInstanceOf(Event.TurnComplete.class, output.events().get(3));
        assertInstanceOf(Event.Result.class, output.events().get(4));
        assertInstanceOf(Event.Error.class, output.events().get(5));
        assertInstanceOf(Event.PermissionRequest.class, output.events().get(6));
    }

    @Test
    void turnComplete_fields() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var turn = (Event.TurnComplete) output.events().get(3);
        assertEquals("end_turn", turn.stopReason());
        assertEquals(0L, turn.turnIndex());
        assertNotNull(turn.usage());
        assertEquals(80, turn.usage().inputTokens());
        assertEquals(40, turn.usage().outputTokens());
    }

    @Test
    void turnComplete_nullStopReason_roundTrips() throws Exception {
        var line =
                "{\"type\":\"turn_complete\",\"stop_reason\":null,\"turn_index\":3,\"usage\":null}";
        var evt = MAPPER.readValue(line, Event.class);
        var turn = assertInstanceOf(Event.TurnComplete.class, evt);
        assertNull(turn.stopReason());
        assertEquals(3L, turn.turnIndex());
        assertNull(turn.usage());
    }

    @Test
    void initEvent_fields() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var init = (Event.Init) output.events().get(0);
        assertEquals("sonnet", init.model());
        assertEquals(List.of("Bash", "Read"), init.tools());
        assertEquals("/home/user", init.workingDirectory());
    }

    @Test
    void assistantMessage_contentBlocks() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var msg = (Event.AssistantMessage) output.events().get(1);
        assertEquals(1, msg.content().size());
        assertInstanceOf(ContentBlock.Text.class, msg.content().get(0));
        var text = (ContentBlock.Text) msg.content().get(0);
        assertEquals("Hello!", text.text());
        assertNotNull(msg.usage());
        assertEquals(100, msg.usage().inputTokens());
    }

    @Test
    void toolExecution_fields() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var tool = (Event.ToolExecution) output.events().get(2);
        assertEquals("Bash", tool.toolName());
        assertEquals("tool_123", tool.toolId());
        assertTrue(tool.result().success());
        assertEquals("hello", tool.result().output());
    }

    @Test
    void errorEvent_fields() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var err = (Event.Error) output.events().get(5);
        assertEquals("oops", err.message());
    }

    @Test
    void permissionRequest_fields() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var perm = (Event.PermissionRequest) output.events().get(6);
        assertEquals("Bash", perm.toolName());
        assertTrue(perm.granted());
    }

    @Test
    void ndjsonEvent_parses() throws Exception {
        var line = """
                {"type":"init","model":"opus","tools":[],"working_directory":null,"metadata":{}}""";
        var evt = MAPPER.readValue(line, Event.class);
        assertNotNull(evt);
        assertInstanceOf(Event.Init.class, evt);
        assertEquals("opus", ((Event.Init) evt).model());
    }
}
