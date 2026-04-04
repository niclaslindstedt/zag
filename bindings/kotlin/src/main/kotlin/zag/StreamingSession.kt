package zag

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.io.Closeable

/**
 * A live streaming session with piped stdin and stdout.
 */
class StreamingSession internal constructor(
    private val process: Process,
) : Closeable {

    /** Send a raw NDJSON line to the agent's stdin. */
    fun send(message: String) {
        process.outputStream.bufferedWriter().let { writer ->
            writer.write(message)
            writer.newLine()
            writer.flush()
        }
    }

    /** Send a user message to the agent. */
    fun sendUserMessage(content: String) {
        val msg = Json.encodeToString(buildJsonObject {
            put("type", "user_message")
            put("content", content)
        })
        send(msg)
    }

    /** Close stdin to signal no more input. */
    fun closeInput() {
        process.outputStream.close()
    }

    /** Async stream of parsed Event objects from stdout. */
    fun events(): Flow<Event> = flow {
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
    }.flowOn(Dispatchers.IO)

    /** Wait for the process to exit. Throws ZagException on non-zero exit. */
    suspend fun wait(): Unit = withContext(Dispatchers.IO) {
        closeInput()

        val stderr = StringBuilder()
        val stderrThread = Thread {
            process.errorStream.bufferedReader().forEachLine { stderr.appendLine(it) }
        }.apply { isDaemon = true; start() }

        val exitCode = process.waitFor()
        stderrThread.join()

        if (exitCode != 0) {
            throw ZagException(
                "zag exited with code $exitCode",
                exitCode,
                stderr.toString(),
            )
        }
    }

    override fun close() {
        process.destroyForcibly()
    }
}
