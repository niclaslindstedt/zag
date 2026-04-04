package io.zag;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.util.Iterator;
import java.util.Map;
import java.util.NoSuchElementException;

/** A live streaming session with piped stdin and stdout. */
public class StreamingSession implements AutoCloseable {

    private static final ObjectMapper MAPPER = ZagProcess.MAPPER;

    private final Process process;
    private final BufferedWriter stdin;
    private final BufferedReader stdout;

    StreamingSession(Process process) {
        this.process = process;
        this.stdin =
                new BufferedWriter(
                        new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));
        this.stdout =
                new BufferedReader(
                        new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));
    }

    /** Send a raw NDJSON line to the agent's stdin. */
    public void send(String message) throws IOException {
        stdin.write(message);
        stdin.newLine();
        stdin.flush();
    }

    /** Send a user message to the agent. */
    public void sendUserMessage(String content) throws IOException {
        String msg = MAPPER.writeValueAsString(Map.of("type", "user_message", "content", content));
        send(msg);
    }

    /** Close stdin to signal no more input. */
    public void closeInput() throws IOException {
        stdin.close();
    }

    /** Return an iterator over parsed Event objects from stdout. */
    public Iterable<Event> events() {
        return () ->
                new Iterator<>() {
                    private Event next;
                    private boolean done;

                    @Override
                    public boolean hasNext() {
                        if (done) return false;
                        if (next != null) return true;
                        try {
                            while (true) {
                                String line = stdout.readLine();
                                if (line == null) {
                                    done = true;
                                    return false;
                                }
                                String trimmed = line.trim();
                                if (trimmed.isEmpty()) continue;
                                try {
                                    next = MAPPER.readValue(trimmed, Event.class);
                                    return true;
                                } catch (Exception e) {
                                    // Skip unparseable lines
                                }
                            }
                        } catch (IOException e) {
                            done = true;
                            return false;
                        }
                    }

                    @Override
                    public Event next() {
                        if (!hasNext()) throw new NoSuchElementException();
                        Event result = next;
                        next = null;
                        return result;
                    }
                };
    }

    /** Wait for the process to exit. Throws ZagException on non-zero exit. */
    public void await() throws ZagException, InterruptedException {
        try {
            stdin.close();
        } catch (IOException ignored) {
        }

        int exitCode = process.waitFor();
        if (exitCode != 0) {
            String stderr;
            try {
                stderr =
                        new String(
                                process.getErrorStream().readAllBytes(), StandardCharsets.UTF_8);
            } catch (IOException e) {
                stderr = "";
            }
            throw new ZagException("zag exited with code " + exitCode, exitCode, stderr);
        }
    }

    @Override
    public void close() {
        process.destroyForcibly();
    }
}
