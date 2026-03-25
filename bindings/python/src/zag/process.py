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
