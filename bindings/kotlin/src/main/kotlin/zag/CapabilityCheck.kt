package zag

import java.util.concurrent.ConcurrentHashMap

/**
 * Provider capability validation for the [ZagBuilder].
 *
 * Before spawning the `zag` CLI, the builder validates feature-gated
 * options ([ZagBuilder.execStreaming], [ZagBuilder.worktree],
 * [ZagBuilder.sandbox], [ZagBuilder.systemPrompt], [ZagBuilder.addDir],
 * [ZagBuilder.maxTurns]) against the capability declarations exposed by
 * `zag discover`. When a caller configures an option that the selected
 * provider does not support, the preflight raises
 * [ZagFeatureUnsupportedException] with a message listing the providers
 * that do support the feature.
 */
object CapabilityCheck {

    /** Capability feature keys the builder can gate on. */
    object FeatureKeys {
        const val STREAMING_INPUT = "streaming_input"
        const val WORKTREE = "worktree"
        const val SANDBOX = "sandbox"
        const val SYSTEM_PROMPT = "system_prompt"
        const val ADD_DIRS = "add_dirs"
        const val MAX_TURNS = "max_turns"
    }

    /** A capability-gated builder option. */
    data class Requirement(val method: String, val feature: String, val isSet: Boolean)

    private val capabilityCache = ConcurrentHashMap<String, List<ProviderCapability>>()

    private fun isFeatureSupported(features: Features, key: String): Boolean = when (key) {
        FeatureKeys.STREAMING_INPUT -> features.streamingInput.supported
        FeatureKeys.WORKTREE -> features.worktree.supported
        FeatureKeys.SANDBOX -> features.sandbox.supported
        FeatureKeys.SYSTEM_PROMPT -> features.systemPrompt.supported
        FeatureKeys.ADD_DIRS -> features.addDirs.supported
        FeatureKeys.MAX_TURNS -> features.maxTurns.supported
        // Unknown key — treat as supported so we never falsely block.
        else -> true
    }

    /**
     * Fetch and cache the full provider capability matrix for a given
     * `zag` binary. Capabilities are compiled into the binary, so the
     * cache lives for the life of the process.
     */
    private suspend fun loadCapabilities(bin: String): List<ProviderCapability> {
        capabilityCache[bin]?.let { return it }
        val caps = ZagDiscover.getAllCapabilities(bin)
        capabilityCache[bin] = caps
        return caps
    }

    /**
     * Validate that every active feature requirement is supported by the
     * configured provider. No-op when [provider] is `null` (so the CLI's
     * default-provider behavior is preserved) or when no requirements are
     * active. If the `zag discover` call itself fails, the preflight
     * silently returns so the subsequent CLI invocation can surface the
     * real error.
     *
     * @throws ZagFeatureUnsupportedException on the first unsupported feature.
     */
    suspend fun check(bin: String, provider: String?, requirements: List<Requirement>) {
        val active = requirements.filter { it.isSet }
        if (active.isEmpty() || provider == null) return

        val caps = try {
            loadCapabilities(bin)
        } catch (e: Exception) {
            // If `zag discover` can't be reached, skip the preflight — the
            // subsequent CLI invocation will surface the real error.
            return
        }

        val providerCap = caps.firstOrNull { it.provider == provider } ?: return

        for (req in active) {
            if (isFeatureSupported(providerCap.features, req.feature)) continue
            val supported = caps
                .filter { isFeatureSupported(it.features, req.feature) }
                .map { it.provider }
            val suffix = if (supported.isEmpty()) {
                " No providers currently support this feature."
            } else {
                " Supported providers: ${supported.joinToString(", ")}"
            }
            throw ZagFeatureUnsupportedException(
                message = "Provider '$provider' does not support ${req.feature} " +
                    "(required by ${req.method}).$suffix",
                provider = provider,
                feature = req.feature,
                method = req.method,
                supportedProviders = supported,
            )
        }
    }

    /** Inject capabilities into the cache for testing. */
    internal fun setCapabilitiesForTesting(bin: String, caps: List<ProviderCapability>) {
        capabilityCache[bin] = caps
    }

    /** Clear the capability cache for testing. */
    internal fun clearCapabilityCache() {
        capabilityCache.clear()
    }
}
