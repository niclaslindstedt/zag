package io.zag;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Capability preflight for feature-gated builder methods.
 *
 * <p>Mirrors the {@link VersionCheck} pattern: terminal builder methods call
 * {@link #check(String, String, List)} before spawning a subprocess so that
 * incompatible provider/feature combinations surface as a typed
 * {@link ZagFeatureUnsupportedException} rather than a cryptic
 * "zag exited with code 1".
 */
public final class CapabilityCheck {

    private CapabilityCheck() {}

    /** Cache of {@code (bin, provider) -> ProviderCapability}. */
    private static final ConcurrentHashMap<String, ProviderCapability> CAP_CACHE = new ConcurrentHashMap<>();

    /** Cache of {@code bin -> List<ProviderCapability>} for the full matrix. */
    private static final ConcurrentHashMap<String, List<ProviderCapability>> ALL_CAPS_CACHE = new ConcurrentHashMap<>();

    /** A feature requirement: builder method, capability field, and whether it is set. */
    public record Requirement(String method, String feature, boolean isSet) {}

    /**
     * Load and cache the capability declaration for {@code (bin, provider)}.
     */
    static ProviderCapability loadCapability(String bin, String provider) throws ZagException {
        String key = bin + "::" + provider;
        ProviderCapability cached = CAP_CACHE.get(key);
        if (cached != null) return cached;
        ProviderCapability cap = ZagDiscover.getCapability(provider, bin);
        CAP_CACHE.put(key, cap);
        return cap;
    }

    /** Load and cache the full capability matrix for {@code bin}. */
    static List<ProviderCapability> loadAllCapabilities(String bin) throws ZagException {
        List<ProviderCapability> cached = ALL_CAPS_CACHE.get(bin);
        if (cached != null) return cached;
        List<ProviderCapability> all = ZagDiscover.getAllCapabilities(bin);
        ALL_CAPS_CACHE.put(bin, all);
        return all;
    }

    /**
     * Check that {@code provider} supports every active requirement. Skips
     * silently when {@code provider} is {@code null} (auto-detect) or
     * {@code "mock"}. Throws {@link ZagFeatureUnsupportedException} on the
     * first unsupported requirement.
     */
    public static void check(String bin, String provider, List<Requirement> requirements)
            throws ZagException {
        List<Requirement> active = requirements.stream()
                .filter(Requirement::isSet)
                .toList();
        if (active.isEmpty()) return;
        if (provider == null || provider.equals("mock")) return;

        ProviderCapability cap = loadCapability(bin, provider);
        ProviderCapability.Features features = cap.features();

        for (Requirement r : active) {
            ProviderCapability.FeatureSupport fs = featureSupport(features, r.feature());
            if (fs != null && fs.supported()) continue;

            // Gather providers that do support this feature.
            List<String> supported = new ArrayList<>();
            try {
                List<ProviderCapability> all = loadAllCapabilities(bin);
                for (ProviderCapability pc : all) {
                    ProviderCapability.FeatureSupport pfs = featureSupport(pc.features(), r.feature());
                    if (pfs != null && pfs.supported()) {
                        supported.add(pc.provider());
                    }
                }
            } catch (ZagException ignored) {
                // If we can't fetch the full matrix, still raise with an empty list.
            }
            throw new ZagFeatureUnsupportedException(r.method(), r.feature(), provider, supported);
        }
    }

    /** Resolve a feature name to its {@link ProviderCapability.FeatureSupport} field. */
    private static ProviderCapability.FeatureSupport featureSupport(
            ProviderCapability.Features f, String feature) {
        if (f == null) return null;
        return switch (feature) {
            case "interactive" -> f.interactive();
            case "non_interactive" -> f.nonInteractive();
            case "resume" -> f.resume();
            case "resume_with_prompt" -> f.resumeWithPrompt();
            case "json_output" -> f.jsonOutput();
            case "stream_json" -> f.streamJson();
            case "json_schema" -> f.jsonSchema();
            case "input_format" -> f.inputFormat();
            case "streaming_input" -> f.streamingInput();
            case "worktree" -> f.worktree();
            case "sandbox" -> f.sandbox();
            case "system_prompt" -> f.systemPrompt();
            case "auto_approve" -> f.autoApprove();
            case "review" -> f.review();
            case "add_dirs" -> f.addDirs();
            case "max_turns" -> f.maxTurns();
            default -> null;
        };
    }

    // -- Test helpers --------------------------------------------------------

    /** Inject a single provider's capability for testing. */
    static void setCapabilityForTesting(String bin, String provider, ProviderCapability cap) {
        CAP_CACHE.put(bin + "::" + provider, cap);
    }

    /** Inject the full capability matrix for a bin (also primes per-provider cache). */
    static void setAllCapabilitiesForTesting(String bin, List<ProviderCapability> caps) {
        ALL_CAPS_CACHE.put(bin, caps);
        for (ProviderCapability c : caps) {
            CAP_CACHE.put(bin + "::" + c.provider(), c);
        }
    }

    /** Clear all cached capability data. */
    static void clearCapabilityCache() {
        CAP_CACHE.clear();
        ALL_CAPS_CACHE.clear();
    }
}
