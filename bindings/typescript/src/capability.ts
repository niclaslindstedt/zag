import {
  ZagError,
  ZagFeatureUnsupportedError,
  type Features,
  type ProviderCapability,
} from "./types.js";
import { getCapability, getAllCapabilities } from "./discover.js";

/** Keys of the `Features` struct that can gate a builder method. */
export type FeatureKey = keyof Features;

/** A single feature-support requirement evaluated at preflight time. */
export interface FeatureRequirement {
  /** Display name for the requiring method, e.g. `"execStreaming()"`. */
  method: string;
  /** Field on the `Features` struct, e.g. `"streaming_input"`. */
  feature: FeatureKey;
  /** Whether this requirement is active (the user configured the option). */
  isSet: boolean;
}

/** Per-(bin, provider) capability cache. */
const capabilityCache = new Map<string, ProviderCapability>();
/** Per-bin full capability matrix cache (used for the supported-providers list). */
const allCapabilitiesCache = new Map<string, ProviderCapability[]>();

function cacheKey(bin: string, provider: string): string {
  return `${bin}::${provider}`;
}

/**
 * Load a provider capability via `zag discover`, memoised per `(bin, provider)`.
 * Wraps errors from `getCapability` in `ZagError` so preflight failures look
 * consistent with `checkVersion()`.
 */
async function loadCapability(
  bin: string,
  provider: string,
): Promise<ProviderCapability> {
  const key = cacheKey(bin, provider);
  const cached = capabilityCache.get(key);
  if (cached) return cached;
  try {
    const cap = await getCapability(provider, bin);
    capabilityCache.set(key, cap);
    return cap;
  } catch (err) {
    if (err instanceof ZagError) throw err;
    throw new ZagError(
      `Failed to load capability for provider '${provider}': ${String(err)}`,
      null,
      "",
    );
  }
}

/** Load the full capability matrix, memoised per `bin`. */
async function loadAllCapabilities(
  bin: string,
): Promise<ProviderCapability[]> {
  const cached = allCapabilitiesCache.get(bin);
  if (cached) return cached;
  try {
    const caps = await getAllCapabilities(bin);
    allCapabilitiesCache.set(bin, caps);
    // Warm the per-provider cache as well so subsequent `loadCapability`
    // calls on the same bin don't trigger additional subprocesses.
    for (const c of caps) {
      capabilityCache.set(cacheKey(bin, c.provider), c);
    }
    return caps;
  } catch (err) {
    if (err instanceof ZagError) throw err;
    throw new ZagError(
      `Failed to load provider capabilities: ${String(err)}`,
      null,
      "",
    );
  }
}

/** Compute the list of providers that natively or wrapper-support a feature. */
function supportedProvidersFor(
  caps: ProviderCapability[],
  feature: FeatureKey,
): string[] {
  return caps
    .filter((c) => c.features[feature]?.supported)
    .map((c) => c.provider);
}

/**
 * Check that every active requirement is supported by the configured provider.
 *
 * Throws `ZagFeatureUnsupportedError` on the first unsupported feature so the
 * caller gets a clear, typed error before any subprocess is spawned.
 *
 * Silently returns when:
 * - no requirement is active,
 * - `provider` is `undefined` (auto-detect — the bindings can't predict which
 *   provider the CLI will ultimately pick),
 * - `provider` is `"mock"` (test stand-in without capability data).
 */
export async function checkCapability(
  bin: string,
  provider: string | undefined,
  requirements: FeatureRequirement[],
): Promise<void> {
  const active = requirements.filter((r) => r.isSet);
  if (active.length === 0) return;
  if (!provider || provider === "mock") return;

  const cap = await loadCapability(bin, provider);

  for (const req of active) {
    const support = cap.features[req.feature];
    if (support && support.supported) continue;

    // Build the "supported providers" list lazily, only when we need it.
    const all = await loadAllCapabilities(bin);
    const supported = supportedProvidersFor(all, req.feature);
    throw new ZagFeatureUnsupportedError(
      req.method,
      String(req.feature),
      provider,
      supported,
    );
  }
}

/** @internal Inject a capability into the cache for testing. */
export function _setCapabilityForTesting(
  bin: string,
  provider: string,
  cap: ProviderCapability,
): void {
  capabilityCache.set(cacheKey(bin, provider), cap);
}

/** @internal Inject the full capability matrix into the cache for testing. */
export function _setAllCapabilitiesForTesting(
  bin: string,
  caps: ProviderCapability[],
): void {
  allCapabilitiesCache.set(bin, caps);
  for (const c of caps) {
    capabilityCache.set(cacheKey(bin, c.provider), c);
  }
}

/** @internal Clear the capability caches for testing. */
export function _clearCapabilityCache(): void {
  capabilityCache.clear();
  allCapabilitiesCache.clear();
}
