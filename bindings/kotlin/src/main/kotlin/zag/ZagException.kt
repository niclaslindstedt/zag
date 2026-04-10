package zag

/**
 * Exception thrown when the zag process fails.
 */
open class ZagException(
    message: String,
    val exitCode: Int?,
    val stderr: String,
) : RuntimeException(message)

/**
 * Thrown by the capability preflight when a builder method is called for a
 * feature the configured provider does not support. Raised before any
 * subprocess is spawned so callers can catch it distinctly from a runtime
 * [ZagException].
 */
class ZagFeatureUnsupportedException(
    val method: String,
    val feature: String,
    val provider: String,
    val supportedProviders: List<String>,
) : ZagException(buildMessage(method, feature, provider, supportedProviders), null, "") {

    companion object {
        private fun buildMessage(
            method: String,
            feature: String,
            provider: String,
            supportedProviders: List<String>,
        ): String {
            val supported = if (supportedProviders.isEmpty()) "(none)" else supportedProviders.joinToString(", ")
            return "$method is not supported by provider '$provider' " +
                "(feature: $feature). Supported providers: $supported"
        }
    }
}
