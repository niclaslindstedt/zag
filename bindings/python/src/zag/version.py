"""CLI version detection and compatibility checking for zag bindings."""

from __future__ import annotations

import asyncio
from typing import NamedTuple

from .types import ZagError

# Minimum CLI version required for each feature (only post-initial-release features).
MIN_VERSIONS: dict[str, str] = {
    "env": "0.6.0",
    "mcp_config": "0.6.0",
}


class SemVer(NamedTuple):
    major: int
    minor: int
    patch: int


def parse_semver(version: str) -> SemVer:
    """Parse a semver string like ``"0.6.0"`` into a numeric tuple."""
    parts = version.strip().split(".")
    if len(parts) != 3:
        raise ZagError(
            f'Could not parse version "{version}": expected format "X.Y.Z"',
            None,
            "",
        )
    try:
        return SemVer(int(parts[0]), int(parts[1]), int(parts[2]))
    except ValueError:
        raise ZagError(
            f'Could not parse version "{version}": non-numeric components',
            None,
            "",
        )


def compare_semver(a: SemVer, b: SemVer) -> int:
    """Compare two semver tuples. Returns -1 if a < b, 0 if equal, 1 if a > b."""
    if a < b:
        return -1
    if a > b:
        return 1
    return 0


# Cached detected versions keyed by binary path.
_version_cache: dict[str, str] = {}


async def detect_version(bin: str) -> str:
    """Detect the CLI version by running ``{bin} --version``.

    Result is cached per binary path.
    """
    cached = _version_cache.get(bin)
    if cached is not None:
        return cached

    try:
        proc = await asyncio.create_subprocess_exec(
            bin,
            "--version",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await proc.communicate()
    except OSError as exc:
        raise ZagError(
            f"Could not detect zag CLI version: failed to run '{bin} --version'. "
            f"Ensure zag is installed and on your PATH, or set ZAG_BIN. ({exc})",
            None,
            "",
        )

    if proc.returncode != 0:
        raise ZagError(
            f"Could not detect zag CLI version: '{bin} --version' "
            f"exited with code {proc.returncode}",
            proc.returncode,
            "",
        )

    output = stdout.decode().strip()
    # Expected format: "zag-cli 0.6.0" or just "0.6.0"
    version_str = output.split()[-1] if output else ""

    try:
        parse_semver(version_str)
    except ZagError:
        raise ZagError(
            f'Could not parse zag CLI version from output: "{output}". '
            'Expected format: "zag-cli X.Y.Z"',
            None,
            "",
        )

    _version_cache[bin] = version_str
    return version_str


class VersionRequirement:
    """A feature requirement with method name, minimum version, and whether it is set."""

    __slots__ = ("method", "version", "is_set")

    def __init__(self, method: str, version: str, *, is_set: bool) -> None:
        self.method = method
        self.version = version
        self.is_set = is_set


async def check_version(
    bin: str, requirements: list[VersionRequirement]
) -> None:
    """Check that the installed CLI version satisfies all configured requirements.

    Raises ``ZagError`` if any requirement is not met.
    """
    active = [r for r in requirements if r.is_set]
    if not active:
        return

    detected = await detect_version(bin)
    detected_sv = parse_semver(detected)

    failures = [
        r for r in active if compare_semver(detected_sv, parse_semver(r.version)) < 0
    ]

    if not failures:
        return

    if len(failures) == 1:
        f = failures[0]
        raise ZagError(
            f"{f.method} requires zag CLI >= {f.version}, "
            f"but the installed version is {detected}. Please update the zag binary.",
            None,
            "",
        )

    lines = [f"  - {f.method} requires >= {f.version}" for f in failures]
    raise ZagError(
        "The following methods require a newer zag CLI version:\n"
        + "\n".join(lines)
        + f"\nInstalled version: {detected}. Please update the zag binary.",
        None,
        "",
    )


def _set_version_for_testing(bin: str, version: str) -> None:
    """Inject a version into the cache for testing."""
    _version_cache[bin] = version


def _clear_version_cache() -> None:
    """Clear the version cache for testing."""
    _version_cache.clear()
