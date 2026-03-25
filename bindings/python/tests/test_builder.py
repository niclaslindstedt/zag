"""Tests for the zag Python binding."""

import json

from zag import ZagBuilder, ZagError
from zag.types import (
    AgentOutput,
    AssistantMessageEvent,
    ErrorEvent,
    InitEvent,
    PermissionRequestEvent,
    ResultEvent,
    TextBlock,
    ToolExecutionEvent,
    Usage,
    parse_event,
)


class TestZagBuilder:
    def test_defaults(self) -> None:
        builder = ZagBuilder()
        assert builder._provider is None
        assert builder._model is None
        assert builder._auto_approve is False

    def test_method_chaining(self) -> None:
        builder = (
            ZagBuilder()
            .provider("claude")
            .model("sonnet")
            .system_prompt("You are helpful")
            .root("/tmp/test")
            .auto_approve()
            .add_dir("/extra")
            .verbose()
            .quiet()
            .debug()
            .session_id("abc-123")
        )
        assert builder._provider == "claude"
        assert builder._model == "sonnet"
        assert builder._system_prompt == "You are helpful"
        assert builder._root == "/tmp/test"
        assert builder._auto_approve is True
        assert builder._add_dirs == ["/extra"]
        assert builder._verbose is True
        assert builder._quiet is True
        assert builder._debug is True
        assert builder._session_id == "abc-123"

    def test_json_options(self) -> None:
        builder = ZagBuilder().json_mode().json_schema({"type": "object"})
        assert builder._json is True
        assert builder._json_schema == {"type": "object"}

    def test_json_schema_implies_json(self) -> None:
        builder = ZagBuilder().json_schema({"type": "object"})
        assert builder._json is True

    def test_isolation_modes(self) -> None:
        wt = ZagBuilder().worktree()
        assert wt._worktree is True

        wt_named = ZagBuilder().worktree("my-feature")
        assert wt_named._worktree == "my-feature"

        sb = ZagBuilder().sandbox()
        assert sb._sandbox is True

        sb_named = ZagBuilder().sandbox("my-sandbox")
        assert sb_named._sandbox == "my-sandbox"

    def test_global_args(self) -> None:
        builder = (
            ZagBuilder()
            .provider("gemini")
            .model("large")
            .root("/project")
            .auto_approve()
            .add_dir("/docs")
            .verbose()
            .debug()
            .session_id("sess-1")
        )
        args = builder._global_args()
        assert args == [
            "-p", "gemini",
            "--model", "large",
            "--root", "/project",
            "--auto-approve",
            "--add-dir", "/docs",
            "--verbose",
            "--debug",
            "--session", "sess-1",
        ]

    def test_exec_args_default_json(self) -> None:
        builder = ZagBuilder().provider("claude")
        args = builder._exec_args("hello")
        assert "exec" in args
        assert "-o" in args
        assert "json" in args
        assert "hello" in args

    def test_exec_args_streaming(self) -> None:
        builder = ZagBuilder()
        args = builder._exec_args("hello", streaming=True)
        assert "--json-stream" in args

    def test_worktree_args(self) -> None:
        args = ZagBuilder().worktree()._global_args()
        assert "-w" in args

        args = ZagBuilder().worktree("feat")._global_args()
        assert ["-w", "feat"] == args[-2:]

    def test_sandbox_args(self) -> None:
        args = ZagBuilder().sandbox()._global_args()
        assert "--sandbox" in args

        args = ZagBuilder().sandbox("box1")._global_args()
        assert ["--sandbox", "box1"] == args[-2:]

    def test_bin_override(self) -> None:
        builder = ZagBuilder().bin("/usr/local/bin/zag")
        assert builder._bin == "/usr/local/bin/zag"


class TestZagError:
    def test_attributes(self) -> None:
        err = ZagError("test error", 1, "stderr output")
        assert str(err) == "test error"
        assert err.exit_code == 1
        assert err.stderr == "stderr output"
        assert isinstance(err, Exception)


class TestAgentOutput:
    def test_from_dict(self) -> None:
        raw = {
            "agent": "claude",
            "session_id": "sess-123",
            "events": [
                {
                    "type": "init",
                    "model": "sonnet",
                    "tools": ["Bash", "Read"],
                    "working_directory": "/home/user",
                    "metadata": {},
                },
                {
                    "type": "assistant_message",
                    "content": [{"type": "text", "text": "Hello!"}],
                    "usage": {"input_tokens": 100, "output_tokens": 50},
                },
                {
                    "type": "tool_execution",
                    "tool_name": "Bash",
                    "tool_id": "tool_123",
                    "input": {"command": "echo hello"},
                    "result": {
                        "success": True,
                        "output": "hello",
                        "error": None,
                        "data": None,
                    },
                },
                {
                    "type": "result",
                    "success": True,
                    "message": "Done",
                    "duration_ms": 1500,
                    "num_turns": 2,
                },
            ],
            "result": "Hello!",
            "is_error": False,
            "total_cost_usd": 0.01,
            "usage": {"input_tokens": 100, "output_tokens": 50},
        }

        output = AgentOutput.from_dict(raw)

        assert output.agent == "claude"
        assert output.session_id == "sess-123"
        assert len(output.events) == 4
        assert output.result == "Hello!"
        assert output.is_error is False
        assert output.total_cost_usd == 0.01
        assert output.usage is not None
        assert output.usage.input_tokens == 100

        assert isinstance(output.events[0], InitEvent)
        assert isinstance(output.events[1], AssistantMessageEvent)
        assert isinstance(output.events[2], ToolExecutionEvent)
        assert isinstance(output.events[3], ResultEvent)


class TestEventParsing:
    def test_parse_init(self) -> None:
        data = {
            "type": "init",
            "model": "opus",
            "tools": [],
            "working_directory": None,
            "metadata": {},
        }
        event = parse_event(data)
        assert isinstance(event, InitEvent)
        assert event.model == "opus"

    def test_parse_assistant_message(self) -> None:
        data = {
            "type": "assistant_message",
            "content": [{"type": "text", "text": "Hi"}],
            "usage": None,
        }
        event = parse_event(data)
        assert isinstance(event, AssistantMessageEvent)
        assert len(event.content) == 1
        assert isinstance(event.content[0], TextBlock)
        assert event.content[0].text == "Hi"

    def test_parse_error(self) -> None:
        data = {"type": "error", "message": "oops", "details": None}
        event = parse_event(data)
        assert isinstance(event, ErrorEvent)
        assert event.message == "oops"

    def test_parse_permission_request(self) -> None:
        data = {
            "type": "permission_request",
            "tool_name": "Bash",
            "description": "run cmd",
            "granted": True,
        }
        event = parse_event(data)
        assert isinstance(event, PermissionRequestEvent)
        assert event.granted is True

    def test_parse_unknown_raises(self) -> None:
        try:
            parse_event({"type": "unknown_type"})
            assert False, "Should have raised ValueError"
        except ValueError as e:
            assert "unknown_type" in str(e)

    def test_ndjson_round_trip(self) -> None:
        """Parse NDJSON lines like the streaming output produces."""
        lines = [
            '{"type":"init","model":"opus","tools":[],"working_directory":null,"metadata":{}}',
            '{"type":"error","message":"oops","details":null}',
        ]
        events = [parse_event(json.loads(line)) for line in lines]
        assert len(events) == 2
        assert isinstance(events[0], InitEvent)
        assert isinstance(events[1], ErrorEvent)
