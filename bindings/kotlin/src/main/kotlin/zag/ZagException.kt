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
 * Exception thrown when a builder option requires a provider feature that
 * the configured provider does not support.
 *
 * The builder validates feature-gated options ([ZagBuilder.execStreaming],
 * [ZagBuilder.worktree], [ZagBuilder.sandbox], [ZagBuilder.systemPrompt],
 * [ZagBuilder.addDir], [ZagBuilder.maxTurns]) against the capability
 * declarations exposed by `zag discover` before spawning the CLI, so
 * callers receive a clear, typed error instead of a cryptic runtime exit
 * code.
 */
class ZagFeatureUnsupportedException(
    message: String,
    val provider: String,
    val feature: String,
    val method: String,
    val supportedProviders: List<String>,
) : ZagException(message, null, "")
