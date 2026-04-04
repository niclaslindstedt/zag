package zag

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.withContext

/**
 * Subprocess helpers for invoking the zag CLI.
 */
internal object ZagProcess {

    /** Get the default zag binary path from ZAG_BIN env or "zag". */
    val defaultBin: String
        get() = System.getenv("ZAG_BIN") ?: "zag"

    /** Run zag and return parsed AgentOutput. */
    suspend fun exec(bin: String, args: List<String>): AgentOutput = withContext(Dispatchers.IO) {
        val process = startProcess(bin, args)

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
            ZagJson.decodeFromString<AgentOutput>(stdout)
        } catch (e: Exception) {
            throw ZagException(
                "Failed to parse zag JSON output: ${stdout.take(200)}",
                exitCode,
                stderr,
            )
        }
    }

    /** Run zag in streaming mode, yielding Event objects from NDJSON. */
    fun stream(bin: String, args: List<String>): Flow<Event> = flow {
        val process = startProcess(bin, args)
        val stderr = StringBuilder()

        // Read stderr in a separate thread to prevent blocking
        val stderrThread = Thread {
            process.errorStream.bufferedReader().forEachLine { stderr.appendLine(it) }
        }.apply { isDaemon = true; start() }

        process.inputStream.bufferedReader().useLines { lines ->
            for (line in lines) {
                val trimmed = line.trim()
                if (trimmed.isEmpty()) continue

                val event = try {
                    ZagJson.decodeFromString<Event>(trimmed)
                } catch (_: Exception) {
                    continue
                }
                emit(event)
            }
        }

        val exitCode = process.waitFor()
        stderrThread.join()

        if (exitCode != 0) {
            throw ZagException(
                "zag exited with code $exitCode",
                exitCode,
                stderr.toString(),
            )
        }
    }.flowOn(Dispatchers.IO)

    /** Start a streaming process with piped stdin and stdout. */
    fun startStreamingProcess(bin: String, args: List<String>): StreamingSession {
        val pb = ProcessBuilder(listOf(bin) + args)
            .redirectErrorStream(false)
        pb.redirectInput(ProcessBuilder.Redirect.PIPE)

        val process = pb.start()
            ?: throw ZagException("Failed to start '$bin'", null, "")

        return StreamingSession(process)
    }

    /** Run zag interactively with inherited stdio. */
    suspend fun run(bin: String, args: List<String>): Unit = withContext(Dispatchers.IO) {
        val process = ProcessBuilder(listOf(bin) + args)
            .inheritIO()
            .start()

        val exitCode = process.waitFor()

        if (exitCode != 0) {
            throw ZagException(
                "zag exited with code $exitCode",
                exitCode,
                "",
            )
        }
    }

    private fun startProcess(bin: String, args: List<String>): Process {
        val pb = ProcessBuilder(listOf(bin) + args)
            .redirectErrorStream(false)
        return pb.start()
            ?: throw ZagException("Failed to start '$bin'", null, "")
    }
}
