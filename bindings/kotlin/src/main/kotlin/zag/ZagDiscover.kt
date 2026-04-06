package zag

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.builtins.ListSerializer

/**
 * Provider and model discovery functions for zag.
 */
object ZagDiscover {

    /**
     * List all available provider names.
     *
     * @param bin Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
     */
    suspend fun listProviders(bin: String? = null): List<String> {
        val caps = getAllCapabilities(bin)
        return caps.map { it.provider }
    }

    /**
     * Get capability declarations for a specific provider.
     *
     * @param provider Provider name (e.g. "claude", "codex", "gemini", "copilot", "ollama").
     * @param bin Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
     */
    suspend fun getCapability(provider: String, bin: String? = null): ProviderCapability {
        val b = bin ?: ZagProcess.defaultBin
        return discoverExec(b, listOf("-p", provider), ProviderCapability.serializer())
    }

    /**
     * Get capability declarations for all providers.
     *
     * @param bin Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
     */
    suspend fun getAllCapabilities(bin: String? = null): List<ProviderCapability> {
        val b = bin ?: ZagProcess.defaultBin
        return discoverExec(b, emptyList(), ListSerializer(ProviderCapability.serializer()))
    }

    /**
     * Resolve a model alias for a given provider.
     *
     * Size aliases (`small`/`s`, `medium`/`m`/`default`, `large`/`l`/`max`) are
     * resolved to the provider-specific model. Non-alias names pass through unchanged.
     *
     * @param provider Provider name.
     * @param model Model name or alias to resolve.
     * @param bin Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
     */
    suspend fun resolveModel(provider: String, model: String, bin: String? = null): ResolvedModel {
        val b = bin ?: ZagProcess.defaultBin
        return discoverExec(b, listOf("-p", provider, "--resolve", model), ResolvedModel.serializer())
    }

    /**
     * Run `zag discover` with the given args and parse JSON output.
     */
    private suspend fun <T> discoverExec(
        bin: String,
        args: List<String>,
        deserializer: kotlinx.serialization.DeserializationStrategy<T>,
    ): T = withContext(Dispatchers.IO) {
        val fullArgs = listOf("discover") + args + "--json"
        val pb = ProcessBuilder(listOf(bin) + fullArgs)
            .redirectErrorStream(false)
        val process = pb.start()
            ?: throw ZagException("Failed to start '$bin'", null, "")

        val stdout = process.inputStream.bufferedReader().readText()
        val stderr = process.errorStream.bufferedReader().readText()
        val exitCode = process.waitFor()

        if (exitCode != 0) {
            throw ZagException(
                "zag exited with code $exitCode: ${stderr.ifEmpty { stdout }}",
                exitCode,
                stderr,
            )
        }

        try {
            ZagJson.decodeFromString(deserializer, stdout)
        } catch (e: Exception) {
            throw ZagException(
                "Failed to parse zag JSON output: ${stdout.take(200)}",
                exitCode,
                stderr,
            )
        }
    }
}
