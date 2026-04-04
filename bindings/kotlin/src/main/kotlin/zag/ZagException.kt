package zag

/**
 * Exception thrown when the zag process fails.
 */
class ZagException(
    message: String,
    val exitCode: Int?,
    val stderr: String,
) : RuntimeException(message)
