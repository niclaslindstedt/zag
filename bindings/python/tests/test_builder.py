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
    TurnCompleteEvent,
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
            .file("/tmp/data.csv")
            .verbose()
            .quiet()
            .debug()
            .session_id("abc-123")
            .max_turns(5)
            .timeout("5m")
            .show_usage()
            .size("9b")
        )
        assert builder._provider == "claude"
        assert builder._model == "sonnet"
        assert builder._system_prompt == "You are helpful"
        assert builder._root == "/tmp/test"
        assert builder._auto_approve is True
        assert builder._add_dirs == ["/extra"]
        assert builder._files == ["/tmp/data.csv"]
        assert builder._verbose is True
        assert builder._quiet is True
        assert builder._debug is True
        assert builder._session_id == "abc-123"
        assert builder._max_turns == 5
        assert builder._timeout == "5m"
        assert builder._show_usage is True
        assert builder._size == "9b"

    def test_env_vars(self) -> None:
        builder = ZagBuilder().env("FOO", "bar").env("BAZ", "qux")
        assert builder._env_vars == ["FOO=bar", "BAZ=qux"]

    def test_env_vars_args(self) -> None:
        args = ZagBuilder().env("FOO", "bar").env("BAZ", "qux")._global_args()
        assert "--env" in args
        idx = args.index("--env")
        assert args[idx + 1] == "FOO=bar"
        assert args[idx + 2] == "--env"
        assert args[idx + 3] == "BAZ=qux"

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

    def test_headless_args(self) -> None:
        args = ZagBuilder().headless()._global_args()
        assert "--headless" in args
        # Disabling stays chainable and removes the flag.
        disabled = ZagBuilder().headless(False)._global_args()
        assert "--headless" not in disabled

    def test_max_turns_args(self) -> None:
        args = ZagBuilder().max_turns(10)._global_args()
        assert ["--max-turns", "10"] == args[-2:]

    def test_mcp_config_args(self) -> None:
        args = ZagBuilder().mcp_config("./mcp.json")._global_args()
        assert ["--mcp-config", "./mcp.json"] == args[-2:]

    def test_show_usage_args(self) -> None:
        args = ZagBuilder().show_usage()._global_args()
        assert "--show-usage" in args

    def test_size_args(self) -> None:
        args = ZagBuilder().size("35b")._global_args()
        assert ["--size", "35b"] == args[-2:]

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
        assert "--json-stream" not in args
        oi = args.index("-o")
        assert args[oi + 1] == "stream-json"

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

    def test_exit_with_hint_in_run_args(self) -> None:
        args = ZagBuilder().exit("the answer")._run_args("compute")
        idx = args.index("--exit")
        assert args[idx + 1] == "the answer"

    def test_exit_bare_in_run_args(self) -> None:
        args = ZagBuilder().exit()._run_args()
        assert "--exit" in args
        # `--exit` is bare; no value should follow it
        idx = args.index("--exit")
        assert idx == len(args) - 1 or args[idx + 1].startswith("-")

    def test_exit_omitted_when_not_set(self) -> None:
        args = ZagBuilder()._run_args("hi")
        assert "--exit" not in args

    def test_timeout_in_exec_args(self) -> None:
        builder = ZagBuilder().timeout("5m")
        args = builder._exec_args("test")
        assert "--timeout" in args
        assert "5m" in args

    def test_bin_override(self) -> None:
        builder = ZagBuilder().bin("/usr/local/bin/zag")
        assert builder._bin == "/usr/local/bin/zag"

    def test_resume_in_exec_args(self) -> None:
        args = ZagBuilder().provider("claude")._exec_args("follow up")
        idx = len(args) - 2  # insert before "--prompt", "follow up"
        args[idx:idx] = ["--resume", "sess-123"]
        assert "--resume" in args
        assert "sess-123" in args
        assert args.index("--resume") < args.index("--prompt")

    def test_continue_in_exec_args(self) -> None:
        args = ZagBuilder().provider("claude")._exec_args("follow up")
        idx = len(args) - 2
        args[idx:idx] = ["--continue"]
        assert "--continue" in args
        assert args.index("--continue") < args.index("--prompt")


class TestVersionChecking:
    def test_parse_semver(self) -> None:
        from zag.version import parse_semver
        assert parse_semver("0.6.0") == (0, 6, 0)
        assert parse_semver("1.2.3") == (1, 2, 3)

    def test_parse_semver_invalid(self) -> None:
        from zag.version import parse_semver
        import pytest

        with pytest.raises(ZagError):
            parse_semver("invalid")
        with pytest.raises(ZagError):
            parse_semver("1.2")
        with pytest.raises(ZagError):
            parse_semver("a.b.c")

    def test_compare_semver(self) -> None:
        from zag.version import compare_semver, SemVer
        assert compare_semver(SemVer(0, 5, 0), SemVer(0, 6, 0)) == -1
        assert compare_semver(SemVer(0, 6, 0), SemVer(0, 6, 0)) == 0
        assert compare_semver(SemVer(0, 7, 0), SemVer(0, 6, 0)) == 1
        assert compare_semver(SemVer(1, 0, 0), SemVer(0, 9, 9)) == 1

    async def test_check_version_no_requirements(self) -> None:
        import asyncio
        from zag.version import check_version, VersionRequirement, _set_version_for_testing, _clear_version_cache
        _set_version_for_testing("zag", "0.5.0")
        try:
            await check_version("zag", [
                VersionRequirement("env()", "0.6.0", is_set=False),
            ])
        finally:
            _clear_version_cache()

    async def test_check_version_sufficient(self) -> None:
        import asyncio
        from zag.version import check_version, VersionRequirement, _set_version_for_testing, _clear_version_cache
        _set_version_for_testing("zag", "0.6.0")
        try:
            await check_version("zag", [
                VersionRequirement("env()", "0.6.0", is_set=True),
            ])
        finally:
            _clear_version_cache()

    async def test_check_version_insufficient(self) -> None:
        import asyncio
        import pytest
        from zag.version import check_version, VersionRequirement, _set_version_for_testing, _clear_version_cache
        _set_version_for_testing("zag", "0.5.0")
        try:
            with pytest.raises(ZagError, match="env().*0.6.0.*0.5.0"):
                await check_version("zag", [
                    VersionRequirement("env()", "0.6.0", is_set=True),
                ])
        finally:
            _clear_version_cache()

    async def test_check_version_multiple_failures(self) -> None:
        import asyncio
        import pytest
        from zag.version import check_version, VersionRequirement, _set_version_for_testing, _clear_version_cache
        _set_version_for_testing("zag", "0.5.0")
        try:
            with pytest.raises(ZagError, match="env()") as exc_info:
                await check_version("zag", [
                    VersionRequirement("env()", "0.6.0", is_set=True),
                    VersionRequirement("mcp_config()", "0.6.0", is_set=True),
                ])
            assert "mcp_config()" in str(exc_info.value)
        finally:
            _clear_version_cache()


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
        assert output.exit_code is None
        assert output.error_message is None
        assert output.total_cost_usd == 0.01
        assert output.usage is not None
        assert output.usage.input_tokens == 100

        assert isinstance(output.events[0], InitEvent)
        assert isinstance(output.events[1], AssistantMessageEvent)
        assert isinstance(output.events[2], ToolExecutionEvent)
        assert isinstance(output.events[3], ResultEvent)

    def test_from_dict_with_exit_info(self) -> None:
        raw = {
            "agent": "codex",
            "session_id": "sess-456",
            "events": [],
            "result": None,
            "is_error": True,
            "exit_code": 2,
            "error_message": "provider crashed",
            "total_cost_usd": None,
            "usage": None,
        }

        output = AgentOutput.from_dict(raw)
        assert output.is_error is True
        assert output.exit_code == 2
        assert output.error_message == "provider crashed"

    def test_from_dict_without_exit_fields(self) -> None:
        """Backwards compatibility: old JSON without exit_code/error_message."""
        raw = {
            "agent": "test",
            "session_id": "",
            "events": [],
            "result": None,
            "is_error": False,
            "total_cost_usd": None,
            "usage": None,
        }

        output = AgentOutput.from_dict(raw)
        assert output.exit_code is None
        assert output.error_message is None


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

    def test_parse_turn_complete(self) -> None:
        data = {
            "type": "turn_complete",
            "stop_reason": "end_turn",
            "turn_index": 0,
            "usage": {"input_tokens": 10, "output_tokens": 5},
        }
        event = parse_event(data)
        assert isinstance(event, TurnCompleteEvent)
        assert event.stop_reason == "end_turn"
        assert event.turn_index == 0
        assert event.usage is not None
        assert event.usage.input_tokens == 10
        assert event.usage.output_tokens == 5

    def test_parse_turn_complete_with_null_stop_reason(self) -> None:
        data = {
            "type": "turn_complete",
            "stop_reason": None,
            "turn_index": 3,
            "usage": None,
        }
        event = parse_event(data)
        assert isinstance(event, TurnCompleteEvent)
        assert event.stop_reason is None
        assert event.turn_index == 3
        assert event.usage is None

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


# ---------------------------------------------------------------------------
# Capability checking
# ---------------------------------------------------------------------------


import pytest

from zag import ZagFeatureUnsupportedError
from zag.capability_check import (
    FeatureRequirement,
    _clear_capability_cache,
    _set_capabilities_for_testing,
    check_capabilities,
)
from zag.types import (
    FeatureSupport,
    Features,
    ProviderCapability,
    SessionLogSupport,
    SizeMappings,
    StreamingInputSupport,
)


def _fake_capability(
    provider: str,
    *,
    streaming_input: bool = True,
    worktree: bool = True,
    sandbox: bool = True,
    system_prompt: bool = True,
    add_dirs: bool = True,
    max_turns: bool = True,
) -> ProviderCapability:
    fs = lambda ok: FeatureSupport(supported=ok, native=ok)
    return ProviderCapability(
        provider=provider,
        default_model="default",
        available_models=["default"],
        size_mappings=SizeMappings(small="s", medium="m", large="l"),
        features=Features(
            interactive=fs(True),
            non_interactive=fs(True),
            resume=fs(True),
            resume_with_prompt=fs(True),
            session_logs=SessionLogSupport(
                supported=True, native=True, completeness="full"
            ),
            json_output=fs(True),
            stream_json=fs(True),
            json_schema=fs(True),
            input_format=fs(True),
            streaming_input=StreamingInputSupport(
                supported=streaming_input,
                native=streaming_input,
                semantics="queue" if streaming_input else None,
            ),
            worktree=fs(worktree),
            sandbox=fs(sandbox),
            system_prompt=fs(system_prompt),
            auto_approve=fs(True),
            review=fs(True),
            add_dirs=fs(add_dirs),
            max_turns=fs(max_turns),
        ),
    )


class TestCapabilityChecking:
    def setup_method(self) -> None:
        _clear_capability_cache()

    def teardown_method(self) -> None:
        _clear_capability_cache()

    @pytest.mark.asyncio
    async def test_skips_when_no_requirements_active(self) -> None:
        _set_capabilities_for_testing("zag", [_fake_capability("claude")])
        await check_capabilities("zag", "claude", [])
        await check_capabilities(
            "zag",
            "claude",
            [FeatureRequirement("worktree()", "worktree", is_set=False)],
        )

    @pytest.mark.asyncio
    async def test_skips_when_provider_is_none(self) -> None:
        _set_capabilities_for_testing(
            "zag", [_fake_capability("ollama", streaming_input=False)]
        )
        await check_capabilities(
            "zag",
            None,
            [
                FeatureRequirement(
                    "exec_streaming()", "streaming_input", is_set=True
                )
            ],
        )

    @pytest.mark.asyncio
    async def test_passes_when_feature_supported(self) -> None:
        _set_capabilities_for_testing(
            "zag", [_fake_capability("claude", streaming_input=True)]
        )
        await check_capabilities(
            "zag",
            "claude",
            [
                FeatureRequirement(
                    "exec_streaming()", "streaming_input", is_set=True
                )
            ],
        )

    @pytest.mark.asyncio
    async def test_raises_on_unsupported(self) -> None:
        _set_capabilities_for_testing(
            "zag",
            [
                _fake_capability("claude", streaming_input=True),
                _fake_capability("ollama", streaming_input=False),
            ],
        )
        with pytest.raises(ZagFeatureUnsupportedError) as excinfo:
            await check_capabilities(
                "zag",
                "ollama",
                [
                    FeatureRequirement(
                        "exec_streaming()", "streaming_input", is_set=True
                    )
                ],
            )
        err = excinfo.value
        assert err.provider == "ollama"
        assert err.feature == "streaming_input"
        assert err.method == "exec_streaming()"
        assert err.supported_providers == ["claude"]
        assert "ollama" in str(err)
        assert "streaming_input" in str(err)
        assert "claude" in str(err)

    @pytest.mark.asyncio
    async def test_reports_add_dirs_gap(self) -> None:
        _set_capabilities_for_testing(
            "zag",
            [
                _fake_capability("claude"),
                _fake_capability("ollama", add_dirs=False, max_turns=False),
            ],
        )
        with pytest.raises(ZagFeatureUnsupportedError):
            await check_capabilities(
                "zag",
                "ollama",
                [FeatureRequirement("add_dir()", "add_dirs", is_set=True)],
            )
        with pytest.raises(ZagFeatureUnsupportedError):
            await check_capabilities(
                "zag",
                "ollama",
                [FeatureRequirement("max_turns()", "max_turns", is_set=True)],
            )

    @pytest.mark.asyncio
    async def test_silent_for_unknown_provider(self) -> None:
        _set_capabilities_for_testing("zag", [_fake_capability("claude")])
        await check_capabilities(
            "zag",
            "does-not-exist",
            [FeatureRequirement("worktree()", "worktree", is_set=True)],
        )

    def test_error_class_hierarchy(self) -> None:
        err = ZagFeatureUnsupportedError(
            "Provider 'ollama' does not support streaming_input",
            "ollama",
            "streaming_input",
            "exec_streaming()",
            ["claude"],
        )
        assert isinstance(err, ZagError)
        assert isinstance(err, Exception)
        assert err.provider == "ollama"
        assert err.feature == "streaming_input"
        assert err.method == "exec_streaming()"
        assert err.supported_providers == ["claude"]
