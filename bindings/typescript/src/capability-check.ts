import type {
  Features,
  ProviderCapability,
} from "./types.js";
import { ZagFeatureUnsupportedError } from "./types.js";
import { getAllCapabilities } from "./discover.js";

/**
 * A capability-gated builder option. The builder collects one of these per
 * configured option before each terminal method call, and the capability
 * checker validates every active requirement against the provider's feature
 * declarations from `zag discover`.
 */
export interface FeatureRequirement {
  /** User-facing builder method name (e.g., `"execStreaming()"`). */
  method: string;
  /** Capability feature key (e.g., `"streaming_input"`). */
  feature: FeatureKey;
  /** Whether the option is active for this invocation. */
  isSet: boolean;
}

/** Feature keys on `Features` that the builder knows how to gate on. */
export type FeatureKey =
  | "streaming_input"
  | "worktree"
  | "sandbox"
  | "system_prompt"
  | "add_dirs"
  | "max_turns";

/**
 * Return `true` if the provider supports this feature (native or via the
 * wrapper).
 */
function featureSupported(features: Features, key: FeatureKey): boolean {
  switch (key) {
    case "streaming_input":
      return features.streaming_input.supported;
    case "worktree":
      return features.worktree.supported;
    case "sandbox":
      return features.sandbox.supported;
    case "system_prompt":
      return features.system_prompt.supported;
    case "add_dirs":
      return features.add_dirs.supported;
    case "max_turns":
      return features.max_turns.supported;
  }
}

/** Cached capability lookups keyed by binary path. */
const capabilityCache = new Map<string, ProviderCapability[]>();

/**
 * Fetch and cache the full provider capability matrix for a given `zag`
 * binary. The result is cached indefinitely (capabilities are compiled into
 * the binary, so they only change when the binary changes).
 */
async function loadCapabilities(
  bin: string,
): Promise<ProviderCapability[]> {
  const cached = capabilityCache.get(bin);
  if (cached) return cached;
  const caps = await getAllCapabilities(bin);
  capabilityCache.set(bin, caps);
  return caps;
}

/**
 * Validate that all active feature requirements are supported by the
 * configured provider. If no provider is set, validation is skipped and the
 * CLI's default-provider behavior is preserved.
 *
 * Throws {@link ZagFeatureUnsupportedError} on the first unsupported feature;
 * the error's `supportedProviders` field lists the providers that do support
 * it, so callers can present actionable guidance.
 */
export async function checkCapabilities(
  bin: string,
  provider: string | undefined,
  requirements: FeatureRequirement[],
): Promise<void> {
  const active = requirements.filter((r) => r.isSet);
  if (active.length === 0 || !provider) return;

  let caps: ProviderCapability[];
  try {
    caps = await loadCapabilities(bin);
  } catch {
    // If we can't reach `zag discover`, skip the preflight check — the
    // subsequent CLI invocation will surface the real error.
    return;
  }

  const providerCap = caps.find((c) => c.provider === provider);
  if (!providerCap) return;

  for (const req of active) {
    if (featureSupported(providerCap.features, req.feature)) continue;
    const supported = caps
      .filter((c) => featureSupported(c.features, req.feature))
      .map((c) => c.provider);
    const suffix =
      supported.length > 0
        ? ` Supported providers: ${supported.join(", ")}`
        : " No providers currently support this feature.";
    throw new ZagFeatureUnsupportedError(
      `Provider '${provider}' does not support ${req.feature} ` +
        `(required by ${req.method}).${suffix}`,
      provider,
      req.feature,
      req.method,
      supported,
    );
  }
}

/** @internal Inject capabilities into the cache for testing. */
export function _setCapabilitiesForTesting(
  bin: string,
  caps: ProviderCapability[],
): void {
  capabilityCache.set(bin, caps);
}

/** @internal Clear the capability cache for testing. */
export function _clearCapabilityCache(): void {
  capabilityCache.clear();
}
