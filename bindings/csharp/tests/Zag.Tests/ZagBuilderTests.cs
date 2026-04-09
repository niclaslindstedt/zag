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
            .File("/tmp/data.csv")
            .Verbose()
            .Debug()
            .SessionId("sess-1")
            .Timeout("5m");

        var args = builder.BuildGlobalArgs();

        Assert.Equal(
            new[]
            {
                "-p", "gemini",
                "--model", "large",
                "--root", "/project",
                "--auto-approve",
                "--add-dir", "/docs",
                "--file", "/tmp/data.csv",
                "--verbose",
                "--debug",
                "--session", "sess-1"
            },
            args);
    }

    [Fact]
    public void Builder_EnvVars_ProduceCorrectArgs()
    {
        var builder = new ZagBuilder()
            .Env("FOO", "bar")
            .Env("BAZ", "qux");

        var args = builder.BuildGlobalArgs();

        Assert.Contains("--env", args);
        var idx = args.IndexOf("--env");
        Assert.Equal("FOO=bar", args[idx + 1]);
        Assert.Equal("--env", args[idx + 2]);
        Assert.Equal("BAZ=qux", args[idx + 3]);
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
    public void Builder_McpConfig()
    {
        var builder = new ZagBuilder().McpConfig("./mcp.json");
        var args = builder.BuildGlobalArgs();
        Assert.Equal(new[] { "--mcp-config", "./mcp.json" }, args);
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

    [Fact]
    public void Timeout_IncludedInExecArgs()
    {
        var builder = new ZagBuilder().Timeout("5m");
        var args = builder.BuildExecArgs("test");
        Assert.Contains("--timeout", args);
        Assert.Contains("5m", args);
    }

    [Fact]
    public void Resume_IncludedInExecArgs()
    {
        var args = new ZagBuilder().Provider("claude").BuildExecArgs("follow up");
        args.Insert(args.Count - 1, "--resume");
        args.Insert(args.Count - 1, "sess-123");
        Assert.Contains("--resume", args);
        Assert.Contains("sess-123", args);
        Assert.True(args.IndexOf("--resume") < args.IndexOf("follow up"));
    }

    [Fact]
    public void Continue_IncludedInExecArgs()
    {
        var args = new ZagBuilder().Provider("claude").BuildExecArgs("follow up");
        args.Insert(args.Count - 1, "--continue");
        Assert.Contains("--continue", args);
        Assert.True(args.IndexOf("--continue") < args.IndexOf("follow up"));
    }
}

public class VersionCheckTests
{
    [Fact]
    public void ParseSemver_Valid()
    {
        var v = VersionCheck.ParseSemver("0.6.0");
        Assert.Equal(0, v.Major);
        Assert.Equal(6, v.Minor);
        Assert.Equal(0, v.Patch);
    }

    [Fact]
    public void ParseSemver_Invalid()
    {
        Assert.Throws<ZagException>(() => VersionCheck.ParseSemver("invalid"));
        Assert.Throws<ZagException>(() => VersionCheck.ParseSemver("1.2"));
        Assert.Throws<ZagException>(() => VersionCheck.ParseSemver("a.b.c"));
    }

    [Fact]
    public void SemVer_Comparison()
    {
        var v050 = VersionCheck.ParseSemver("0.5.0");
        var v060 = VersionCheck.ParseSemver("0.6.0");
        var v070 = VersionCheck.ParseSemver("0.7.0");

        Assert.True(v050.CompareTo(v060) < 0);
        Assert.Equal(0, v060.CompareTo(v060));
        Assert.True(v070.CompareTo(v060) > 0);
    }

    [Fact]
    public async Task Check_NoActiveRequirements_Passes()
    {
        VersionCheck.SetVersionForTesting("zag", "0.5.0");
        try
        {
            await VersionCheck.CheckAsync("zag", new[]
            {
                new VersionCheck.Requirement("Env()", "0.6.0", false),
            });
        }
        finally
        {
            VersionCheck.ClearVersionCache();
        }
    }

    [Fact]
    public async Task Check_SufficientVersion_Passes()
    {
        VersionCheck.SetVersionForTesting("zag", "0.6.0");
        try
        {
            await VersionCheck.CheckAsync("zag", new[]
            {
                new VersionCheck.Requirement("Env()", "0.6.0", true),
            });
        }
        finally
        {
            VersionCheck.ClearVersionCache();
        }
    }

    [Fact]
    public async Task Check_InsufficientVersion_Throws()
    {
        VersionCheck.SetVersionForTesting("zag", "0.5.0");
        try
        {
            var ex = await Assert.ThrowsAsync<ZagException>(() =>
                VersionCheck.CheckAsync("zag", new[]
                {
                    new VersionCheck.Requirement("Env()", "0.6.0", true),
                }));
            Assert.Contains("Env()", ex.Message);
            Assert.Contains("0.6.0", ex.Message);
            Assert.Contains("0.5.0", ex.Message);
        }
        finally
        {
            VersionCheck.ClearVersionCache();
        }
    }

    [Fact]
    public async Task Check_MultipleFailures_ReportsAll()
    {
        VersionCheck.SetVersionForTesting("zag", "0.5.0");
        try
        {
            var ex = await Assert.ThrowsAsync<ZagException>(() =>
                VersionCheck.CheckAsync("zag", new[]
                {
                    new VersionCheck.Requirement("Env()", "0.6.0", true),
                    new VersionCheck.Requirement("McpConfig()", "0.6.0", true),
                }));
            Assert.Contains("Env()", ex.Message);
            Assert.Contains("McpConfig()", ex.Message);
        }
        finally
        {
            VersionCheck.ClearVersionCache();
        }
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
        Assert.Null(output.ExitCode);
        Assert.Null(output.ErrorMessage);
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
