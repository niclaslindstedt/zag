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
        assertTrue(args.contains("--json-stream"));
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
        assertEquals(6, output.events().size());
        assertEquals("Hello!", output.result());
        assertFalse(output.isError());
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
        assertInstanceOf(Event.Result.class, output.events().get(3));
        assertInstanceOf(Event.Error.class, output.events().get(4));
        assertInstanceOf(Event.PermissionRequest.class, output.events().get(5));
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
        var err = (Event.Error) output.events().get(4);
        assertEquals("oops", err.message());
    }

    @Test
    void permissionRequest_fields() throws Exception {
        var output = MAPPER.readValue(SAMPLE_JSON, AgentOutput.class);
        var perm = (Event.PermissionRequest) output.events().get(5);
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
