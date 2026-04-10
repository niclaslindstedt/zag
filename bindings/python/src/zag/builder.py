"""Fluent builder for configuring and running zag agent sessions.

Example::

    from zag import ZagBuilder

    output = await ZagBuilder() \\
        .provider("claude") \\
        .model("sonnet") \\
        .auto_approve() \\
        .exec("write a hello world program")

    print(output.result)
"""

from __future__ import annotations

import json
from collections.abc import AsyncGenerator

from .process import default_bin, exec_zag, run_zag, stream_zag, stream_with_input
from .types import AgentOutput, Event
from .version import VersionRequirement, check_version


class ZagBuilder:
    """Fluent builder for configuring and running zag agent sessions."""

    def __init__(self) -> None:
        self._bin: str = default_bin()
        self._provider: str | None = None
        self._model: str | None = None
        self._system_prompt: str | None = None
        self._root: str | None = None
        self._auto_approve: bool = False
        self._add_dirs: list[str] = []
        self._files: list[str] = []
        self._env_vars: list[str] = []
        self._json: bool = False
        self._json_schema: dict | None = None
        self._worktree: str | bool | None = None
        self._sandbox: str | bool | None = None
        self._verbose: bool = False
        self._quiet: bool = False
        self._debug: bool = False
        self._session_id: str | None = None
        self._output_format: str | None = None
        self._input_format: str | None = None
        self._replay_user_messages: bool = False
        self._include_partial_messages: bool = False
        self._max_turns: int | None = None
        self._timeout: str | None = None
        self._mcp_config: str | None = None
        self._show_usage: bool = False
        self._size: str | None = None

    # -- Configuration methods -----------------------------------------------

    def bin(self, path: str) -> ZagBuilder:
        """Override the zag binary path (default: ``ZAG_BIN`` env or ``"zag"``)."""
        self._bin = path
        return self

    def provider(self, p: str) -> ZagBuilder:
        """Set the provider (e.g., ``"claude"``, ``"codex"``, ``"gemini"``)."""
        self._provider = p
        return self

    def model(self, m: str) -> ZagBuilder:
        """Set the model (e.g., ``"sonnet"``, ``"opus"``, ``"small"``)."""
        self._model = m
        return self

    def system_prompt(self, p: str) -> ZagBuilder:
        """Set a system prompt to configure agent behavior."""
        self._system_prompt = p
        return self

    def root(self, r: str) -> ZagBuilder:
        """Set the root directory for the agent to operate in."""
        self._root = r
        return self

    def auto_approve(self, a: bool = True) -> ZagBuilder:
        """Enable auto-approve mode (skip permission prompts)."""
        self._auto_approve = a
        return self

    def add_dir(self, d: str) -> ZagBuilder:
        """Add an additional directory for the agent to include."""
        self._add_dirs.append(d)
        return self

    def file(self, path: str) -> ZagBuilder:
        """Attach a file to the prompt (chainable)."""
        self._files.append(path)
        return self

    def env(self, key: str, value: str) -> ZagBuilder:
        """Add an environment variable for the agent subprocess."""
        self._env_vars.append(f"{key}={value}")
        return self

    def json_mode(self) -> ZagBuilder:
        """Request JSON output from the agent."""
        self._json = True
        return self

    def json_schema(self, s: dict) -> ZagBuilder:
        """Set a JSON schema for structured output validation. Implies ``json_mode()``."""
        self._json_schema = s
        self._json = True
        return self

    def worktree(self, name: str | None = None) -> ZagBuilder:
        """Enable worktree mode with an optional name."""
        self._worktree = name if name is not None else True
        return self

    def sandbox(self, name: str | None = None) -> ZagBuilder:
        """Enable sandbox mode with an optional name."""
        self._sandbox = name if name is not None else True
        return self

    def verbose(self, v: bool = True) -> ZagBuilder:
        """Enable verbose output."""
        self._verbose = v
        return self

    def quiet(self, q: bool = True) -> ZagBuilder:
        """Enable quiet mode."""
        self._quiet = q
        return self

    def debug(self, d: bool = True) -> ZagBuilder:
        """Enable debug logging."""
        self._debug = d
        return self

    def session_id(self, id: str) -> ZagBuilder:
        """Pre-set a session ID (UUID)."""
        self._session_id = id
        return self

    def output_format(self, f: str) -> ZagBuilder:
        """Set the output format (e.g., ``"text"``, ``"json"``, ``"stream-json"``)."""
        self._output_format = f
        return self

    def input_format(self, f: str) -> ZagBuilder:
        """Set the input format (Claude only)."""
        self._input_format = f
        return self

    def replay_user_messages(self, r: bool = True) -> ZagBuilder:
        """Re-emit user messages from stdin on stdout (Claude only)."""
        self._replay_user_messages = r
        return self

    def include_partial_messages(self, i: bool = True) -> ZagBuilder:
        """Include partial message chunks in streaming output (Claude only)."""
        self._include_partial_messages = i
        return self

    def max_turns(self, n: int) -> ZagBuilder:
        """Set the maximum number of agentic turns."""
        self._max_turns = n
        return self

    def timeout(self, t: str) -> ZagBuilder:
        """Set a timeout duration (e.g., ``"30s"``, ``"5m"``, ``"1h"``). Kills the agent if exceeded."""
        self._timeout = t
        return self

    def mcp_config(self, c: str) -> ZagBuilder:
        """Set MCP server config for this invocation: JSON string or file path (Claude only)."""
        self._mcp_config = c
        return self

    def show_usage(self, s: bool = True) -> ZagBuilder:
        """Show token usage statistics (only applies to JSON output mode)."""
        self._show_usage = s
        return self

    def size(self, s: str) -> ZagBuilder:
        """Set the Ollama model parameter size (e.g., ``"2b"``, ``"9b"``, ``"35b"``)."""
        self._size = s
        return self

    # -- Version checking ----------------------------------------------------

    def _version_requirements(self) -> list[VersionRequirement]:
        return [
            VersionRequirement("env()", "0.6.0", is_set=len(self._env_vars) > 0),
            VersionRequirement("mcp_config()", "0.6.0", is_set=self._mcp_config is not None),
        ]

    # -- Arg building --------------------------------------------------------

    def _global_args(self) -> list[str]:
        args: list[str] = []
        if self._provider:
            args.extend(["-p", self._provider])
        if self._model:
            args.extend(["--model", self._model])
        if self._system_prompt:
            args.extend(["--system-prompt", self._system_prompt])
        if self._root:
            args.extend(["--root", self._root])
        if self._auto_approve:
            args.append("--auto-approve")
        for d in self._add_dirs:
            args.extend(["--add-dir", d])
        for f in self._files:
            args.extend(["--file", f])
        for e in self._env_vars:
            args.extend(["--env", e])
        if self._worktree is True:
            args.append("-w")
        elif isinstance(self._worktree, str):
            args.extend(["-w", self._worktree])
        if self._sandbox is True:
            args.append("--sandbox")
        elif isinstance(self._sandbox, str):
            args.extend(["--sandbox", self._sandbox])
        if self._verbose:
            args.append("--verbose")
        if self._quiet:
            args.append("--quiet")
        if self._debug:
            args.append("--debug")
        if self._session_id:
            args.extend(["--session", self._session_id])
        if self._max_turns is not None:
            args.extend(["--max-turns", str(self._max_turns)])
        if self._mcp_config:
            args.extend(["--mcp-config", self._mcp_config])
        if self._show_usage:
            args.append("--show-usage")
        if self._size:
            args.extend(["--size", self._size])
        return args

    def _exec_args(self, prompt: str, *, streaming: bool = False) -> list[str]:
        args = ["exec", *self._global_args()]
        if self._json:
            args.append("--json")
        if self._json_schema:
            args.extend(["--json-schema", json.dumps(self._json_schema)])
        if self._output_format:
            args.extend(["-o", self._output_format])
        elif streaming:
            args.extend(["-o", "stream-json"])
        else:
            # Default to json output for structured parsing
            args.extend(["-o", "json"])
        if self._input_format:
            args.extend(["-i", self._input_format])
        if self._replay_user_messages:
            args.append("--replay-user-messages")
        if self._include_partial_messages:
            args.append("--include-partial-messages")
        if self._timeout:
            args.extend(["--timeout", self._timeout])
        args.append(prompt)
        return args

    # -- Terminal methods ----------------------------------------------------

    async def exec(self, prompt: str) -> AgentOutput:
        """Run the agent non-interactively and return structured output.

        Example::

            output = await ZagBuilder().provider("claude").exec("say hello")
            print(output.result)
        """
        await check_version(self._bin, self._version_requirements())
        args = self._exec_args(prompt)
        return await exec_zag(self._bin, args)

    async def exec_streaming(self, prompt: str) -> "StreamingSession":
        """Run the agent with streaming input and output (Claude only).

        Returns a StreamingSession for bidirectional communication.

        Example::

            session = await ZagBuilder().provider("claude").exec_streaming("hello")
            await session.send_user_message("do something")
            async for event in session.events():
                print(event.type)
            await session.wait()
        """
        await check_version(self._bin, self._version_requirements())
        from .process import StreamingSession as _StreamingSession

        args = ["exec", *self._global_args()]
        args.extend(["-i", "stream-json"])
        args.extend(["-o", "stream-json"])
        args.append("--replay-user-messages")
        if self._include_partial_messages:
            args.append("--include-partial-messages")
        args.append(prompt)
        return await _StreamingSession.create(self._bin, args)

    async def stream(self, prompt: str) -> AsyncGenerator[Event, None]:
        """Run the agent in streaming mode, yielding events as they arrive.

        Example::

            async for event in await ZagBuilder().provider("claude").stream("analyze"):
                print(event.type)
        """
        await check_version(self._bin, self._version_requirements())
        args = self._exec_args(prompt, streaming=True)
        async for event in stream_zag(self._bin, args):
            yield event

    async def run(self, prompt: str | None = None) -> None:
        """Start an interactive agent session (inherits stdio)."""
        await check_version(self._bin, self._version_requirements())
        args = ["run", *self._global_args()]
        if self._json:
            args.append("--json")
        if self._json_schema:
            args.extend(["--json-schema", json.dumps(self._json_schema)])
        if prompt:
            args.append(prompt)
        await run_zag(self._bin, args)

    async def resume(self, session_id: str) -> None:
        """Resume a previous session by ID."""
        await check_version(self._bin, self._version_requirements())
        args = ["run", *self._global_args(), "--resume", session_id]
        await run_zag(self._bin, args)

    async def continue_last(self) -> None:
        """Resume the most recent session."""
        await check_version(self._bin, self._version_requirements())
        args = ["run", *self._global_args(), "--continue"]
        await run_zag(self._bin, args)

    async def exec_resume(self, session_id: str, prompt: str) -> AgentOutput:
        """Resume a previous session non-interactively with a follow-up prompt.

        Example::

            output = await ZagBuilder().provider("claude").exec_resume("id", "follow up")
            print(output.result)
        """
        await check_version(self._bin, self._version_requirements())
        args = self._exec_args(prompt)
        idx = len(args) - 1  # prompt is last
        args[idx:idx] = ["--resume", session_id]
        return await exec_zag(self._bin, args)

    async def exec_continue(self, prompt: str) -> AgentOutput:
        """Resume the most recent session non-interactively with a follow-up prompt.

        Example::

            output = await ZagBuilder().provider("claude").exec_continue("follow up")
            print(output.result)
        """
        await check_version(self._bin, self._version_requirements())
        args = self._exec_args(prompt)
        idx = len(args) - 1  # prompt is last
        args[idx:idx] = ["--continue"]
        return await exec_zag(self._bin, args)

    async def stream_resume(
        self, session_id: str, prompt: str
    ) -> AsyncGenerator[Event, None]:
        """Resume a previous session in streaming mode with a follow-up prompt."""
        await check_version(self._bin, self._version_requirements())
        args = self._exec_args(prompt, streaming=True)
        idx = len(args) - 1
        args[idx:idx] = ["--resume", session_id]
        async for event in stream_zag(self._bin, args):
            yield event

    async def stream_continue(self, prompt: str) -> AsyncGenerator[Event, None]:
        """Resume the most recent session in streaming mode with a follow-up prompt."""
        await check_version(self._bin, self._version_requirements())
        args = self._exec_args(prompt, streaming=True)
        idx = len(args) - 1
        args[idx:idx] = ["--continue"]
        async for event in stream_zag(self._bin, args):
            yield event
