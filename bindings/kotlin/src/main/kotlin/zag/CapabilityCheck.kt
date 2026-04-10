package zag

import java.util.concurrent.ConcurrentHashMap

/**
 * Capability preflight for feature-gated builder methods.
 *
 * Mirrors [VersionCheck]: terminal builder methods call [check] before
 * spawning a subprocess so that incompatible provider/feature combinations
 * surface as a typed [ZagFeatureUnsupportedException] instead of a cryptic
 * "zag exited with code 1".
 */
object CapabilityCheck {

    /** A feature requirement: builder method, capability field, and whether it is set. */
    data class Requirement(val method: String, val feature: String, val isSet: Boolean)

    private val capCache = ConcurrentHashMap<String, ProviderCapability>()
    private val allCapsCache = ConcurrentHashMap<String, List<ProviderCapability>>()

    /** Load and cache the capability declaration for `(bin, provider)`. */
    internal suspend fun loadCapability(bin: String, provider: String): ProviderCapability {
        val key = "$bin::$provider"
        capCache[key]?.let { return it }
        val cap = ZagDiscover.getCapability(provider, bin)
        capCache[key] = cap
        return cap
    }

    /** Load and cache the full capability matrix for [bin]. */
    internal suspend fun loadAllCapabilities(bin: String): List<ProviderCapability> {
        allCapsCache[bin]?.let { return it }
        val all = ZagDiscover.getAllCapabilities(bin)
        allCapsCache[bin] = all
        return all
    }

    /**
     * Check that [provider] supports every active requirement. Skips silently
     * when [provider] is `null` (auto-detect) or `"mock"`. Throws
     * [ZagFeatureUnsupportedException] on the first unsupported requirement.
     */
    suspend fun check(bin: String, provider: String?, requirements: List<Requirement>) {
        val active = requirements.filter { it.isSet }
        if (active.isEmpty()) return
        if (provider == null || provider == "mock") return

        val cap = loadCapability(bin, provider)
        for (r in active) {
            val fs = featureSupport(cap.features, r.feature)
            if (fs != null && fs.supported) continue

            val supported = mutableListOf<String>()
            try {
                for (pc in loadAllCapabilities(bin)) {
                    val pfs = featureSupport(pc.features, r.feature)
                    if (pfs != null && pfs.supported) {
                        supported.add(pc.provider)
                    }
                }
            } catch (_: ZagException) {
                // If we can't fetch the full matrix, still raise with an empty list.
            }
            throw ZagFeatureUnsupportedException(r.method, r.feature, provider, supported)
        }
    }

    /** Resolve a feature name to its [FeatureSupport] field. */
    private fun featureSupport(f: Features, feature: String): FeatureSupport? = when (feature) {
        "interactive" -> f.interactive
        "non_interactive" -> f.nonInteractive
        "resume" -> f.resume
        "resume_with_prompt" -> f.resumeWithPrompt
        "json_output" -> f.jsonOutput
        "stream_json" -> f.streamJson
        "json_schema" -> f.jsonSchema
        "input_format" -> f.inputFormat
        "streaming_input" -> f.streamingInput
        "worktree" -> f.worktree
        "sandbox" -> f.sandbox
        "system_prompt" -> f.systemPrompt
        "auto_approve" -> f.autoApprove
        "review" -> f.review
        "add_dirs" -> f.addDirs
        "max_turns" -> f.maxTurns
        else -> null
    }

    // -- Test helpers --------------------------------------------------------

    /** Inject a single provider's capability for testing. */
    internal fun setCapabilityForTesting(bin: String, provider: String, cap: ProviderCapability) {
        capCache["$bin::$provider"] = cap
    }

    /** Inject the full capability matrix for a bin (also primes per-provider cache). */
    internal fun setAllCapabilitiesForTesting(bin: String, caps: List<ProviderCapability>) {
        allCapsCache[bin] = caps
        for (c in caps) {
            capCache["$bin::${c.provider}"] = c
        }
    }

    /** Clear all cached capability data. */
    internal fun clearCapabilityCache() {
        capCache.clear()
        allCapsCache.clear()
    }
}
