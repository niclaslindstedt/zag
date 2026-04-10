"""Tests for the provider-capability preflight helper."""

from __future__ import annotations

import pytest

from zag import ZagBuilder, ZagError, ZagFeatureUnsupportedError
from zag.capability_check import (
    FeatureRequirement,
    _clear_capability_cache,
    _set_all_capabilities_for_testing,
    _set_capability_for_testing,
    check_capability,
)
from zag.types import FeatureSupport, Features, ProviderCapability, SizeMappings
from zag.version import _clear_version_cache, _set_version_for_testing


def fake_cap(provider: str, **overrides: bool) -> ProviderCapability:
    """Build a synthetic ProviderCapability with the given feature overrides."""
    defaults = {
        "interactive": True,
        "non_interactive": True,
        "resume": False,
        "resume_with_prompt": False,
        "json_output": True,
        "stream_json": True,
        "json_schema": False,
        "input_format": False,
        "streaming_input": False,
        "worktree": False,
        "sandbox": False,
        "system_prompt": False,
        "auto_approve": True,
        "review": False,
        "add_dirs": False,
        "max_turns": False,
    }
    defaults.update(overrides)
    kwargs = {
        name: FeatureSupport(supported=supported, native=False)
        for name, supported in defaults.items()
    }
    return ProviderCapability(
        provider=provider,
        default_model="default",
        available_models=[],
        size_mappings=SizeMappings(),
        features=Features(**kwargs),
    )


def prime_caps(bin: str, caps: list[ProviderCapability]) -> None:
    """Seed both the per-(bin,provider) and per-bin caches."""
    _clear_capability_cache()
    _set_all_capabilities_for_testing(bin, caps)


class TestCheckCapability:
    async def test_no_requirements_returns_silently(self) -> None:
        prime_caps("zag", [fake_cap("ollama")])
        try:
            await check_capability("zag", "ollama", [])
        finally:
            _clear_capability_cache()

    async def test_inactive_requirements_return_silently(self) -> None:
        prime_caps("zag", [fake_cap("ollama")])
        try:
            await check_capability(
                "zag",
                "ollama",
                [FeatureRequirement("add_dir()", "add_dirs", is_set=False)],
            )
        finally:
            _clear_capability_cache()

    async def test_none_provider_skips(self) -> None:
        # No cache primed — would raise if we tried to load.
        await check_capability(
            "zag",
            None,
            [FeatureRequirement("add_dir()", "add_dirs", is_set=True)],
        )

    async def test_mock_provider_skips(self) -> None:
        await check_capability(
            "zag",
            "mock",
            [FeatureRequirement("add_dir()", "add_dirs", is_set=True)],
        )

    async def test_supported_feature_passes(self) -> None:
        prime_caps("zag", [fake_cap("claude", streaming_input=True)])
        try:
            await check_capability(
                "zag",
                "claude",
                [
                    FeatureRequirement(
                        "exec_streaming()", "streaming_input", is_set=True
                    )
                ],
            )
        finally:
            _clear_capability_cache()

    async def test_unsupported_feature_raises(self) -> None:
        prime_caps(
            "zag",
            [
                fake_cap("claude", streaming_input=True),
                fake_cap("ollama", streaming_input=False),
            ],
        )
        try:
            with pytest.raises(ZagFeatureUnsupportedError) as exc_info:
                await check_capability(
                    "zag",
                    "ollama",
                    [
                        FeatureRequirement(
                            "exec_streaming()",
                            "streaming_input",
                            is_set=True,
                        )
                    ],
                )
            err = exc_info.value
            assert err.method == "exec_streaming()"
            assert err.feature == "streaming_input"
            assert err.provider == "ollama"
            assert "claude" in err.supported_providers
            assert "ollama" not in err.supported_providers
            assert isinstance(err, ZagError)
        finally:
            _clear_capability_cache()

    async def test_unsupported_with_no_supporters(self) -> None:
        prime_caps("zag", [fake_cap("ollama")])
        try:
            with pytest.raises(ZagFeatureUnsupportedError) as exc_info:
                await check_capability(
                    "zag",
                    "ollama",
                    [FeatureRequirement("sandbox()", "sandbox", is_set=True)],
                )
            assert "(none)" in str(exc_info.value)
        finally:
            _clear_capability_cache()


class TestZagBuilderCapabilityPreflight:
    async def test_add_dir_on_ollama_raises(self) -> None:
        _set_version_for_testing("zag", "9.9.9")
        prime_caps(
            "zag",
            [
                fake_cap("claude", add_dirs=True),
                fake_cap("ollama", add_dirs=False),
            ],
        )
        try:
            builder = ZagBuilder().provider("ollama").add_dir("/extra")
            with pytest.raises(ZagFeatureUnsupportedError) as exc_info:
                await builder.exec("hello")
            assert exc_info.value.method == "add_dir()"
            assert exc_info.value.provider == "ollama"
        finally:
            _clear_capability_cache()
            _clear_version_cache()

    async def test_exec_streaming_on_gemini_raises(self) -> None:
        _set_version_for_testing("zag", "9.9.9")
        prime_caps(
            "zag",
            [
                fake_cap("claude", streaming_input=True),
                fake_cap("gemini", streaming_input=False),
            ],
        )
        try:
            builder = ZagBuilder().provider("gemini")
            with pytest.raises(ZagFeatureUnsupportedError) as exc_info:
                await builder.exec_streaming("hi")
            assert exc_info.value.method == "exec_streaming()"
            assert exc_info.value.provider == "gemini"
            assert "claude" in exc_info.value.supported_providers
        finally:
            _clear_capability_cache()
            _clear_version_cache()

    async def test_worktree_on_supported_provider_passes_preflight(self) -> None:
        """Worktree check passes; the subprocess call then fails because no
        binary exists, but FeatureUnsupported must not fire."""
        _set_version_for_testing("zag", "9.9.9")
        prime_caps("zag", [fake_cap("claude", worktree=True)])
        try:
            builder = ZagBuilder().provider("claude").worktree("feat").bin(
                "/nonexistent/zag"
            )
            # Preflight passes; subprocess then fails with FileNotFoundError.
            with pytest.raises((ZagError, FileNotFoundError)) as exc_info:
                await builder.exec("hi")
            assert not isinstance(exc_info.value, ZagFeatureUnsupportedError)
        finally:
            _clear_capability_cache()
            _clear_version_cache()

    async def test_no_provider_skips_preflight(self) -> None:
        _set_version_for_testing("zag", "9.9.9")
        try:
            # No caps primed; with no provider set, the preflight should skip
            # and the subprocess call proceeds (then fails — not our concern).
            builder = ZagBuilder().add_dir("/extra").bin("/nonexistent/zag")
            with pytest.raises((ZagError, FileNotFoundError)) as exc_info:
                await builder.exec("hi")
            assert not isinstance(exc_info.value, ZagFeatureUnsupportedError)
        finally:
            _clear_capability_cache()
            _clear_version_cache()


class TestZagFeatureUnsupportedError:
    def test_message_format(self) -> None:
        err = ZagFeatureUnsupportedError(
            "exec_streaming()",
            "streaming_input",
            "ollama",
            ["claude"],
        )
        msg = str(err)
        assert "exec_streaming()" in msg
        assert "ollama" in msg
        assert "streaming_input" in msg
        assert "claude" in msg

    def test_empty_supported_list(self) -> None:
        err = ZagFeatureUnsupportedError("sandbox()", "sandbox", "ollama", [])
        assert "(none)" in str(err)

    def test_is_zag_error(self) -> None:
        err = ZagFeatureUnsupportedError("worktree()", "worktree", "x", [])
        assert isinstance(err, ZagError)
