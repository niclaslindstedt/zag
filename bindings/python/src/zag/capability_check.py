"""Provider-capability preflight helper for the zag Python bindings.

Mirrors the ``checkVersion`` pattern in :mod:`zag.version`: every terminal
builder method calls :func:`check_capability` with the list of configured
feature requirements before spawning the real agent subprocess. When a
configured option is unsupported by the pinned provider, a
:class:`~zag.types.ZagFeatureUnsupportedError` is raised with an actionable
message — instead of the user seeing a cryptic ``"zag exited with code 1"``
after the subprocess fails.
"""

from __future__ import annotations

from typing import Any

from .discover import get_all_capabilities, get_capability
from .types import ProviderCapability, ZagError, ZagFeatureUnsupportedError


class FeatureRequirement:
    """A single feature-support requirement evaluated at preflight time."""

    __slots__ = ("method", "feature", "is_set")

    def __init__(self, method: str, feature: str, *, is_set: bool) -> None:
        self.method = method
        self.feature = feature
        self.is_set = is_set


# Per-``(bin, provider)`` capability cache.
_capability_cache: dict[tuple[str, str], ProviderCapability] = {}
# Per-``bin`` full capability matrix cache (used for the supported-providers list).
_all_capabilities_cache: dict[str, list[ProviderCapability]] = {}


async def _load_capability(bin: str, provider: str) -> ProviderCapability:
    cached = _capability_cache.get((bin, provider))
    if cached is not None:
        return cached
    try:
        cap = await get_capability(provider, bin)
    except ZagError:
        raise
    except Exception as exc:  # pragma: no cover - defensive
        raise ZagError(
            f"Failed to load capability for provider '{provider}': {exc}",
            None,
            "",
        ) from exc
    _capability_cache[(bin, provider)] = cap
    return cap


async def _load_all_capabilities(bin: str) -> list[ProviderCapability]:
    cached = _all_capabilities_cache.get(bin)
    if cached is not None:
        return cached
    try:
        caps = await get_all_capabilities(bin)
    except ZagError:
        raise
    except Exception as exc:  # pragma: no cover - defensive
        raise ZagError(
            f"Failed to load provider capabilities: {exc}", None, ""
        ) from exc
    _all_capabilities_cache[bin] = caps
    # Warm the per-provider cache so subsequent loads don't re-spawn discover.
    for c in caps:
        _capability_cache[(bin, c.provider)] = c
    return caps


def _feature_supported(cap: ProviderCapability, feature: str) -> bool:
    support: Any = getattr(cap.features, feature, None)
    return bool(support and getattr(support, "supported", False))


def _supported_providers_for(
    caps: list[ProviderCapability], feature: str
) -> list[str]:
    return [c.provider for c in caps if _feature_supported(c, feature)]


async def check_capability(
    bin: str,
    provider: str | None,
    requirements: list[FeatureRequirement],
) -> None:
    """Check that every active requirement is supported by ``provider``.

    Raises :class:`ZagFeatureUnsupportedError` on the first unsupported
    feature. Returns silently when:

    - no requirement is active,
    - ``provider`` is ``None`` (auto-detect — the bindings can't predict
      which provider the CLI will ultimately pick),
    - ``provider`` is ``"mock"`` (test stand-in without capability data).
    """
    active = [r for r in requirements if r.is_set]
    if not active:
        return
    if provider is None or provider == "mock":
        return

    cap = await _load_capability(bin, provider)

    for req in active:
        if _feature_supported(cap, req.feature):
            continue

        caps = await _load_all_capabilities(bin)
        supported = _supported_providers_for(caps, req.feature)
        raise ZagFeatureUnsupportedError(
            req.method, req.feature, provider, supported
        )


def _set_capability_for_testing(
    bin: str, provider: str, cap: ProviderCapability
) -> None:
    """Inject a capability into the cache for testing."""
    _capability_cache[(bin, provider)] = cap


def _set_all_capabilities_for_testing(
    bin: str, caps: list[ProviderCapability]
) -> None:
    """Inject the full capability matrix into the cache for testing."""
    _all_capabilities_cache[bin] = caps
    for c in caps:
        _capability_cache[(bin, c.provider)] = c


def _clear_capability_cache() -> None:
    """Clear the capability caches for testing."""
    _capability_cache.clear()
    _all_capabilities_cache.clear()
