"""Provider and model discovery functions for zag."""

from __future__ import annotations

import asyncio
import json
import os
from typing import Any

from .types import ProviderCapability, ResolvedModel, ZagError


def _default_bin() -> str:
    return os.environ.get("ZAG_BIN", "zag")


async def _discover_exec(bin: str, args: list[str]) -> Any:
    """Run ``zag discover`` with the given args and parse JSON output."""
    proc = await asyncio.create_subprocess_exec(
        bin,
        "discover",
        *args,
        "--json",
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
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise ZagError(
            f"Failed to parse zag JSON output: {stdout[:200]}",
            proc.returncode,
            stderr,
        ) from exc


async def list_providers(bin: str | None = None) -> list[str]:
    """List all available provider names.

    Args:
        bin: Path to the zag binary (defaults to ``ZAG_BIN`` env or ``"zag"``).
    """
    caps = await get_all_capabilities(bin)
    return [c.provider for c in caps]


async def get_capability(
    provider: str, bin: str | None = None
) -> ProviderCapability:
    """Get capability declarations for a specific provider.

    Args:
        provider: Provider name (e.g. ``"claude"``, ``"codex"``).
        bin: Path to the zag binary.
    """
    b = bin or _default_bin()
    data = await _discover_exec(b, ["-p", provider])
    return ProviderCapability.from_dict(data)


async def get_all_capabilities(
    bin: str | None = None,
) -> list[ProviderCapability]:
    """Get capability declarations for all providers.

    Args:
        bin: Path to the zag binary.
    """
    b = bin or _default_bin()
    data = await _discover_exec(b, [])
    return [ProviderCapability.from_dict(item) for item in data]


async def resolve_model(
    provider: str, model: str, bin: str | None = None
) -> ResolvedModel:
    """Resolve a model alias for a given provider.

    Size aliases (``small``/``s``, ``medium``/``m``/``default``,
    ``large``/``l``/``max``) are resolved to the provider-specific model.
    Non-alias names pass through unchanged.

    Args:
        provider: Provider name.
        model: Model name or alias to resolve.
        bin: Path to the zag binary.
    """
    b = bin or _default_bin()
    data = await _discover_exec(b, ["-p", provider, "--resolve", model])
    return ResolvedModel.from_dict(data)
