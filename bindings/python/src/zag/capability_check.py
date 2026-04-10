"""Provider capability validation for the ZagBuilder.

Before spawning the zag CLI, the builder validates feature-gated options
(``exec_streaming``, ``worktree``, ``sandbox``, ``system_prompt``, ``add_dir``,
``max_turns``) against the capability declarations exposed by
``zag discover``. When a caller configures an option that the selected
provider does not support, the preflight raises
:class:`ZagFeatureUnsupportedError` with a clear message listing the
providers that do support the feature, instead of letting the call fail
later with a cryptic exit code.
"""

from __future__ import annotations

from .discover import get_all_capabilities
from .types import Features, ProviderCapability, ZagFeatureUnsupportedError


class FeatureRequirement:
    """A capability-gated builder option.

    Attributes:
        method: User-facing builder method name (e.g., ``"exec_streaming()"``).
        feature: Capability feature key (e.g., ``"streaming_input"``).
        is_set: Whether the option is active for this invocation.
    """

    __slots__ = ("method", "feature", "is_set")

    def __init__(self, method: str, feature: str, *, is_set: bool) -> None:
        self.method = method
        self.feature = feature
        self.is_set = is_set


def _feature_supported(features: Features, key: str) -> bool:
    """Return ``True`` if the provider supports this feature (native or wrapper)."""
    if key == "streaming_input":
        return features.streaming_input.supported
    if key == "worktree":
        return features.worktree.supported
    if key == "sandbox":
        return features.sandbox.supported
    if key == "system_prompt":
        return features.system_prompt.supported
    if key == "add_dirs":
        return features.add_dirs.supported
    if key == "max_turns":
        return features.max_turns.supported
    # Unknown key — treat as supported so we never falsely block.
    return True


# Cached capability lookups keyed by binary path. Capabilities are compiled
# into the binary, so once fetched they can be reused indefinitely.
_capability_cache: dict[str, list[ProviderCapability]] = {}


async def _load_capabilities(bin: str) -> list[ProviderCapability]:
    cached = _capability_cache.get(bin)
    if cached is not None:
        return cached
    caps = await get_all_capabilities(bin)
    _capability_cache[bin] = caps
    return caps


async def check_capabilities(
    bin: str,
    provider: str | None,
    requirements: list[FeatureRequirement],
) -> None:
    """Validate that every active feature requirement is supported.

    No-ops when ``provider`` is ``None`` (the CLI's default-provider behavior
    is preserved) or when no requirements are active. If the ``zag discover``
    call itself fails, the preflight silently yields so the subsequent CLI
    invocation can surface the real error.

    Raises :class:`ZagFeatureUnsupportedError` on the first unsupported
    feature.
    """
    active = [r for r in requirements if r.is_set]
    if not active or not provider:
        return

    try:
        caps = await _load_capabilities(bin)
    except Exception:
        return

    provider_cap = next((c for c in caps if c.provider == provider), None)
    if provider_cap is None:
        return

    for req in active:
        if _feature_supported(provider_cap.features, req.feature):
            continue
        supported = [
            c.provider
            for c in caps
            if _feature_supported(c.features, req.feature)
        ]
        suffix = (
            f" Supported providers: {', '.join(supported)}"
            if supported
            else " No providers currently support this feature."
        )
        raise ZagFeatureUnsupportedError(
            f"Provider '{provider}' does not support {req.feature} "
            f"(required by {req.method}).{suffix}",
            provider,
            req.feature,
            req.method,
            supported,
        )


def _set_capabilities_for_testing(
    bin: str, caps: list[ProviderCapability]
) -> None:
    """Inject capabilities into the cache for testing."""
    _capability_cache[bin] = caps


def _clear_capability_cache() -> None:
    """Clear the capability cache for testing."""
    _capability_cache.clear()
