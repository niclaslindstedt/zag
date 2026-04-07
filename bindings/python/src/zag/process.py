"""Subprocess helpers for invoking the zag CLI."""

from __future__ import annotations

import asyncio
import json
import os
from collections.abc import AsyncGenerator

from .types import AgentOutput, Event, ZagError, parse_event


def default_bin() -> str:
    """Return the zag binary path (``ZAG_BIN`` env or ``"zag"``)."""
    return os.environ.get("ZAG_BIN", "zag")


async def exec_zag(bin: str, args: list[str]) -> AgentOutput:
    """Run ``zag`` and return parsed :class:`AgentOutput`.

    Raises :class:`ZagError` on non-zero exit.
    """
    proc = await asyncio.create_subprocess_exec(
        bin,
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout_bytes, stderr_bytes = await proc.communicate()
    stdout = stdout_bytes.decode()
    stderr = stderr_bytes.decode()

    if proc.returncode != 0:
        raise ZagError(
            f"zag exited with code {proc.returncode}: {stderr or stdout}",
            proc.returncode,
            stderr,
        )

    try:
        data = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise ZagError(
            f"Failed to parse zag JSON output: {stdout[:200]}",
            proc.returncode,
            stderr,
        ) from exc

    return AgentOutput.from_dict(data)


async def stream_zag(bin: str, args: list[str]) -> AsyncGenerator[Event, None]:
    """Run ``zag`` in streaming mode and yield :class:`Event` objects (NDJSON)."""
    proc = await asyncio.create_subprocess_exec(
        bin,
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    assert proc.stdout is not None
    assert proc.stderr is not None

    while True:
        line = await proc.stdout.readline()
        if not line:
            break
        text = line.decode().strip()
        if not text:
            continue
        try:
            data = json.loads(text)
            yield parse_event(data)
        except (json.JSONDecodeError, ValueError):
            continue

    stderr_bytes = await proc.stderr.read()
    await proc.wait()

    if proc.returncode != 0:
        stderr = stderr_bytes.decode()
        raise ZagError(
            f"zag exited with code {proc.returncode}",
            proc.returncode,
            stderr,
        )


class StreamingSession:
    """A live streaming session with piped stdin and stdout.

    Send NDJSON messages via :meth:`send`, read events via :meth:`events`,
    then call :meth:`wait` when done.
    """

    def __init__(self, proc: asyncio.subprocess.Process) -> None:
        self._proc = proc

    @classmethod
    async def create(cls, bin: str, args: list[str]) -> "StreamingSession":
        proc = await asyncio.create_subprocess_exec(
            bin,
            *args,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        return cls(proc)

    async def send(self, message: str) -> None:
        """Send a raw NDJSON line to the agent's stdin."""
        assert self._proc.stdin is not None
        self._proc.stdin.write((message + "\n").encode())
        await self._proc.stdin.drain()

    async def send_user_message(self, content: str) -> None:
        """Send a user message to the agent."""
        msg = json.dumps({"type": "user_message", "content": content})
        await self.send(msg)

    def close_input(self) -> None:
        """Close stdin to signal no more input."""
        if self._proc.stdin is not None:
            self._proc.stdin.close()

    def terminate(self) -> None:
        """Send SIGTERM to the subprocess."""
        self._proc.terminate()

    @property
    def is_running(self) -> bool:
        """Return True if the subprocess is still running."""
        return self._proc.returncode is None

    async def events(self) -> AsyncGenerator[Event, None]:
        """Async iterator over parsed Event objects from stdout."""
        assert self._proc.stdout is not None
        while True:
            line = await self._proc.stdout.readline()
            if not line:
                break
            text = line.decode().strip()
            if not text:
                continue
            try:
                data = json.loads(text)
                yield parse_event(data)
            except (json.JSONDecodeError, ValueError):
                continue

    async def wait(self) -> None:
        """Wait for the process to exit. Raises ZagError on non-zero exit."""
        self.close_input()
        assert self._proc.stderr is not None
        stderr_bytes = await self._proc.stderr.read()
        await self._proc.wait()
        if self._proc.returncode != 0:
            stderr = stderr_bytes.decode()
            raise ZagError(
                f"zag exited with code {self._proc.returncode}",
                self._proc.returncode,
                stderr,
            )


def stream_with_input(bin: str, args: list[str]) -> StreamingSession:
    """Create a StreamingSession (alias for StreamingSession.create)."""
    # This is a sync wrapper that returns the coroutine; callers should await it
    raise NotImplementedError("Use StreamingSession.create() directly")


async def run_zag(bin: str, args: list[str]) -> None:
    """Run ``zag`` interactively with inherited stdio.

    Raises :class:`ZagError` on non-zero exit.
    """
    proc = await asyncio.create_subprocess_exec(bin, *args)
    await proc.wait()

    if proc.returncode != 0:
        raise ZagError(
            f"zag exited with code {proc.returncode}",
            proc.returncode,
            "",
        )
