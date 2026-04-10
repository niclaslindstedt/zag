package io.zag;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Provider capability validation for the {@link ZagBuilder}.
 *
 * <p>Before spawning the {@code zag} CLI, the builder validates
 * feature-gated options ({@link ZagBuilder#execStreaming},
 * {@link ZagBuilder#worktree}, {@link ZagBuilder#sandbox},
 * {@link ZagBuilder#systemPrompt}, {@link ZagBuilder#addDir},
 * {@link ZagBuilder#maxTurns}) against the capability declarations exposed
 * by {@code zag discover}. When a caller configures an option that the
 * selected provider does not support, the preflight raises
 * {@link ZagFeatureUnsupportedException} with a message listing the
 * providers that do support the feature.
 */
public final class CapabilityCheck {

    private CapabilityCheck() {}

    /** Capability feature keys the builder can gate on. */
    public static final class FeatureKeys {
        public static final String STREAMING_INPUT = "streaming_input";
        public static final String WORKTREE = "worktree";
        public static final String SANDBOX = "sandbox";
        public static final String SYSTEM_PROMPT = "system_prompt";
        public static final String ADD_DIRS = "add_dirs";
        public static final String MAX_TURNS = "max_turns";

        private FeatureKeys() {}
    }

    /** A capability-gated builder option. */
    public record Requirement(String method, String feature, boolean isSet) {}

    private static final ConcurrentHashMap<String, List<ProviderCapability>> CAPABILITY_CACHE =
            new ConcurrentHashMap<>();

    private static boolean isFeatureSupported(ProviderCapability.Features features, String key) {
        return switch (key) {
            case FeatureKeys.STREAMING_INPUT -> features.streamingInput().supported();
            case FeatureKeys.WORKTREE -> features.worktree().supported();
            case FeatureKeys.SANDBOX -> features.sandbox().supported();
            case FeatureKeys.SYSTEM_PROMPT -> features.systemPrompt().supported();
            case FeatureKeys.ADD_DIRS -> features.addDirs().supported();
            case FeatureKeys.MAX_TURNS -> features.maxTurns().supported();
            // Unknown key — treat as supported so we never falsely block.
            default -> true;
        };
    }

    /**
     * Fetch and cache the full provider capability matrix for a given
     * {@code zag} binary. Capabilities are compiled into the binary, so
     * the cache lives for the life of the process.
     */
    private static List<ProviderCapability> loadCapabilities(String bin) throws ZagException {
        List<ProviderCapability> cached = CAPABILITY_CACHE.get(bin);
        if (cached != null) return cached;
        List<ProviderCapability> caps = ZagDiscover.getAllCapabilities(bin);
        CAPABILITY_CACHE.put(bin, caps);
        return caps;
    }

    /**
     * Validate that every active feature requirement is supported by the
     * configured provider. No-op when {@code provider} is {@code null} (so
     * the CLI's default-provider behavior is preserved) or when no
     * requirements are active. If the {@code zag discover} call itself
     * fails, the preflight silently returns so the subsequent CLI
     * invocation can surface the real error.
     *
     * @throws ZagFeatureUnsupportedException on the first unsupported feature.
     */
    public static void check(String bin, String provider, List<Requirement> requirements)
            throws ZagException {
        List<Requirement> active = new ArrayList<>();
        for (Requirement r : requirements) {
            if (r.isSet()) active.add(r);
        }
        if (active.isEmpty() || provider == null) return;

        List<ProviderCapability> caps;
        try {
            caps = loadCapabilities(bin);
        } catch (ZagException e) {
            // If `zag discover` can't be reached, skip the preflight — the
            // subsequent CLI invocation will surface the real error.
            return;
        }

        ProviderCapability providerCap = null;
        for (ProviderCapability c : caps) {
            if (provider.equals(c.provider())) {
                providerCap = c;
                break;
            }
        }
        if (providerCap == null) return;

        for (Requirement req : active) {
            if (isFeatureSupported(providerCap.features(), req.feature())) continue;
            List<String> supported = new ArrayList<>();
            for (ProviderCapability c : caps) {
                if (isFeatureSupported(c.features(), req.feature())) {
                    supported.add(c.provider());
                }
            }
            String suffix = supported.isEmpty()
                    ? " No providers currently support this feature."
                    : " Supported providers: " + String.join(", ", supported);
            throw new ZagFeatureUnsupportedException(
                    "Provider '" + provider + "' does not support " + req.feature()
                            + " (required by " + req.method() + ")." + suffix,
                    provider,
                    req.feature(),
                    req.method(),
                    supported);
        }
    }

    /** Inject capabilities into the cache for testing. */
    static void setCapabilitiesForTesting(String bin, List<ProviderCapability> caps) {
        CAPABILITY_CACHE.put(bin, caps);
    }

    /** Clear the capability cache for testing. */
    static void clearCapabilityCache() {
        CAPABILITY_CACHE.clear();
    }
}
