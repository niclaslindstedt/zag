using System.Text.Json;
using Xunit;
using Zag;

namespace Zag.Tests;

public class ZagBuilderTests
{
    [Fact]
    public void Builder_Defaults_ProduceMinimalArgs()
    {
        var builder = new ZagBuilder();
        var args = builder.BuildGlobalArgs();
        Assert.Empty(args);
    }

    [Fact]
    public void Builder_MethodChaining_SetsAllOptions()
    {
        var builder = new ZagBuilder()
            .Provider("gemini")
            .Model("large")
            .Root("/project")
            .AutoApprove()
            .AddDir("/docs")
            .Verbose()
            .Debug()
            .SessionId("sess-1");

        var args = builder.BuildGlobalArgs();

        Assert.Equal(
            new[]
            {
                "-p", "gemini",
                "--model", "large",
                "--root", "/project",
                "--auto-approve",
                "--add-dir", "/docs",
                "--verbose",
                "--debug",
                "--session", "sess-1"
            },
            args);
    }

    [Fact]
    public void Builder_ExecArgs_DefaultsToJson()
    {
        var builder = new ZagBuilder().Provider("claude");
        var args = builder.BuildExecArgs("hello");

        Assert.Contains("exec", args);
        Assert.Contains("-o", args);
        Assert.Contains("json", args);
        Assert.Contains("hello", args);
    }

    [Fact]
    public void Builder_ExecArgs_Streaming()
    {
        var builder = new ZagBuilder();
        var args = builder.BuildExecArgs("hello", streaming: true);
        Assert.Contains("--json-stream", args);
    }

    [Fact]
    public void Builder_Worktree_NoName()
    {
        var builder = new ZagBuilder().Worktree();
        var args = builder.BuildGlobalArgs();
        Assert.Contains("-w", args);
        Assert.Single(args);
    }

    [Fact]
    public void Builder_Worktree_Named()
    {
        var builder = new ZagBuilder().Worktree("feat");
        var args = builder.BuildGlobalArgs();
        Assert.Equal(new[] { "-w", "feat" }, args);
    }

    [Fact]
    public void Builder_Sandbox_NoName()
    {
        var builder = new ZagBuilder().Sandbox();
        var args = builder.BuildGlobalArgs();
        Assert.Contains("--sandbox", args);
        Assert.Single(args);
    }

    [Fact]
    public void Builder_Sandbox_Named()
    {
        var builder = new ZagBuilder().Sandbox("box1");
        var args = builder.BuildGlobalArgs();
        Assert.Equal(new[] { "--sandbox", "box1" }, args);
    }

    [Fact]
    public void Builder_MaxTurns()
    {
        var builder = new ZagBuilder().MaxTurns(10);
        var args = builder.BuildGlobalArgs();
        Assert.Equal(new[] { "--max-turns", "10" }, args);
    }

    [Fact]
    public void Builder_ShowUsage()
    {
        var builder = new ZagBuilder().ShowUsage();
        var args = builder.BuildGlobalArgs();
        Assert.Equal(new[] { "--show-usage" }, args);
    }

    [Fact]
    public void Builder_Size()
    {
        var builder = new ZagBuilder().Size("35b");
        var args = builder.BuildGlobalArgs();
        Assert.Equal(new[] { "--size", "35b" }, args);
    }
}

public class ZagExceptionTests
{
    [Fact]
    public void Exception_ContainsFields()
    {
        var ex = new ZagException("test error", 1, "stderr output");
        Assert.Equal("test error", ex.Message);
        Assert.Equal(1, ex.ExitCode);
        Assert.Equal("stderr output", ex.Stderr);
        Assert.IsAssignableFrom<Exception>(ex);
    }
}

public class ModelsTests
{
    private static readonly string SampleJson = """
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

    [Fact]
    public void AgentOutput_Deserializes()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson);
        Assert.NotNull(output);
        Assert.Equal("claude", output.Agent);
        Assert.Equal("sess-123", output.SessionId);
        Assert.Equal(6, output.Events.Count);
        Assert.Equal("Hello!", output.Result);
        Assert.False(output.IsError);
        Assert.Equal(0.01, output.TotalCostUsd);
        Assert.NotNull(output.Usage);
        Assert.Equal(100, output.Usage.InputTokens);
    }

    [Fact]
    public void Events_DeserializeCorrectTypes()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson)!;

        Assert.IsType<InitEvent>(output.Events[0]);
        Assert.IsType<AssistantMessageEvent>(output.Events[1]);
        Assert.IsType<ToolExecutionEvent>(output.Events[2]);
        Assert.IsType<ResultEvent>(output.Events[3]);
        Assert.IsType<ErrorEvent>(output.Events[4]);
        Assert.IsType<PermissionRequestEvent>(output.Events[5]);
    }

    [Fact]
    public void InitEvent_Fields()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson)!;
        var init = (InitEvent)output.Events[0];
        Assert.Equal("sonnet", init.Model);
        Assert.Equal(new[] { "Bash", "Read" }, init.Tools);
        Assert.Equal("/home/user", init.WorkingDirectory);
    }

    [Fact]
    public void AssistantMessage_ContentBlocks()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson)!;
        var msg = (AssistantMessageEvent)output.Events[1];
        Assert.Single(msg.Content);
        var text = Assert.IsType<TextBlock>(msg.Content[0]);
        Assert.Equal("Hello!", text.Text);
        Assert.NotNull(msg.Usage);
        Assert.Equal(100, msg.Usage.InputTokens);
    }

    [Fact]
    public void ToolExecution_Fields()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson)!;
        var tool = (ToolExecutionEvent)output.Events[2];
        Assert.Equal("Bash", tool.ToolName);
        Assert.Equal("tool_123", tool.ToolId);
        Assert.True(tool.Result.Success);
        Assert.Equal("hello", tool.Result.Output);
    }

    [Fact]
    public void ErrorEvent_Fields()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson)!;
        var err = (ErrorEvent)output.Events[4];
        Assert.Equal("oops", err.Message);
    }

    [Fact]
    public void PermissionRequest_Fields()
    {
        var output = JsonSerializer.Deserialize<AgentOutput>(SampleJson)!;
        var perm = (PermissionRequestEvent)output.Events[5];
        Assert.Equal("Bash", perm.ToolName);
        Assert.True(perm.Granted);
    }

    [Fact]
    public void NdjsonEvent_Parses()
    {
        var line = """{"type":"init","model":"opus","tools":[],"working_directory":null,"metadata":{}}""";
        var evt = JsonSerializer.Deserialize<Event>(line);
        Assert.NotNull(evt);
        Assert.IsType<InitEvent>(evt);
        Assert.Equal("opus", ((InitEvent)evt).Model);
    }
}
