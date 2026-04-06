package zag

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.concurrent.ConcurrentHashMap

/** CLI version detection and compatibility checking for zag bindings. */
object VersionCheck {

    /** A parsed semver tuple. */
    data class SemVer(val major: Int, val minor: Int, val patch: Int) : Comparable<SemVer> {
        override fun compareTo(other: SemVer): Int {
            major.compareTo(other.major).let { if (it != 0) return it }
            minor.compareTo(other.minor).let { if (it != 0) return it }
            return patch.compareTo(other.patch)
        }
    }

    /** A feature requirement with method name, minimum version, and whether it is set. */
    data class Requirement(val method: String, val version: String, val isSet: Boolean)

    private val versionCache = ConcurrentHashMap<String, String>()

    /** Parse a semver string like "0.6.0" into a numeric tuple. */
    fun parseSemver(version: String): SemVer {
        val parts = version.trim().split(".")
        if (parts.size != 3) {
            throw ZagException(
                "Could not parse version \"$version\": expected format \"X.Y.Z\"",
                null, "")
        }
        try {
            return SemVer(parts[0].toInt(), parts[1].toInt(), parts[2].toInt())
        } catch (e: NumberFormatException) {
            throw ZagException(
                "Could not parse version \"$version\": non-numeric components",
                null, "")
        }
    }

    /**
     * Detect the CLI version by running `{bin} --version`.
     * Result is cached per binary path.
     */
    suspend fun detectVersion(bin: String): String {
        versionCache[bin]?.let { return it }

        return withContext(Dispatchers.IO) {
            val process = try {
                ProcessBuilder(bin, "--version").start()
            } catch (e: Exception) {
                throw ZagException(
                    "Could not detect zag CLI version: failed to run '$bin --version'. " +
                        "Ensure zag is installed and on your PATH, or set ZAG_BIN. (${e.message})",
                    null, "")
            }

            val stdout = process.inputStream.bufferedReader().readText().trim()
            val exitCode = process.waitFor()

            if (exitCode != 0) {
                throw ZagException(
                    "Could not detect zag CLI version: '$bin --version' exited with code $exitCode",
                    exitCode, "")
            }

            val parts = stdout.split("\\s+".toRegex())
            val versionStr = parts.lastOrNull() ?: ""

            // Validate it parses
            parseSemver(versionStr)

            versionCache[bin] = versionStr
            versionStr
        }
    }

    /**
     * Check that the installed CLI version satisfies all configured requirements.
     * Throws [ZagException] if any requirement is not met.
     */
    suspend fun check(bin: String, requirements: List<Requirement>) {
        val active = requirements.filter { it.isSet }
        if (active.isEmpty()) return

        val detected = detectVersion(bin)
        val detectedSv = parseSemver(detected)

        val failures = active.filter { detectedSv < parseSemver(it.version) }
        if (failures.isEmpty()) return

        if (failures.size == 1) {
            val f = failures[0]
            throw ZagException(
                "${f.method} requires zag CLI >= ${f.version}, " +
                    "but the installed version is $detected. Please update the zag binary.",
                null, "")
        }

        val lines = failures.joinToString("\n") { "  - ${it.method} requires >= ${it.version}" }
        throw ZagException(
            "The following methods require a newer zag CLI version:\n$lines\n" +
                "Installed version: $detected. Please update the zag binary.",
            null, "")
    }

    /** Inject a version into the cache for testing. */
    internal fun setVersionForTesting(bin: String, version: String) {
        versionCache[bin] = version
    }

    /** Clear the version cache for testing. */
    internal fun clearVersionCache() {
        versionCache.clear()
    }
}
