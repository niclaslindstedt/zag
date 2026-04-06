"""Tests for the zag discover module."""

import pytest
from zag import (
    get_all_capabilities,
    get_capability,
    list_providers,
    resolve_model,
)


# These tests require the zag binary to be built and available in PATH.

@pytest.mark.asyncio
async def test_list_providers():
    providers = await list_providers()
    assert len(providers) >= 5
    assert "claude" in providers
    assert "codex" in providers
    assert "gemini" in providers
    assert "copilot" in providers
    assert "ollama" in providers


@pytest.mark.asyncio
async def test_get_capability():
    cap = await get_capability("claude")
    assert cap.provider == "claude"
    assert len(cap.available_models) > 0
    assert cap.features.interactive.supported is True


@pytest.mark.asyncio
async def test_get_all_capabilities():
    caps = await get_all_capabilities()
    assert len(caps) >= 5
    names = [c.provider for c in caps]
    assert "claude" in names


@pytest.mark.asyncio
async def test_resolve_model_alias():
    rm = await resolve_model("claude", "small")
    assert rm.input == "small"
    assert rm.resolved == "haiku"
    assert rm.is_alias is True
    assert rm.provider == "claude"


@pytest.mark.asyncio
async def test_resolve_model_passthrough():
    rm = await resolve_model("claude", "opus")
    assert rm.input == "opus"
    assert rm.resolved == "opus"
    assert rm.is_alias is False
