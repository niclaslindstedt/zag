package io.zag;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.NoSuchElementException;

/** Subprocess helpers for invoking the zag CLI. */
public final class ZagProcess {

    static final ObjectMapper MAPPER =
            new ObjectMapper().configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    private ZagProcess() {}

    /** Get the default zag binary path from ZAG_BIN env or "zag". */
    public static String defaultBin() {
        String env = System.getenv("ZAG_BIN");
        return env != null ? env : "zag";
    }

    /** Run zag and return parsed AgentOutput. */
    public static AgentOutput exec(String bin, List<String> args) throws ZagException {
        List<String> command = buildCommand(bin, args);
        try {
            ProcessBuilder pb = new ProcessBuilder(command);
            pb.redirectErrorStream(false);
            Process process = pb.start();

            String stdout =
                    new String(process.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
            String stderr =
                    new String(process.getErrorStream().readAllBytes(), StandardCharsets.UTF_8);

            int exitCode = process.waitFor();
            if (exitCode != 0) {
                String msg = stderr.isEmpty() ? stdout : stderr;
                throw new ZagException(
                        "zag exited with code " + exitCode + ": " + msg, exitCode, stderr);
            }

            return MAPPER.readValue(stdout, AgentOutput.class);
        } catch (ZagException e) {
            throw e;
        } catch (IOException | InterruptedException e) {
            throw new ZagException("Failed to run zag: " + e.getMessage(), null, "");
        }
    }

    /** Run zag in streaming mode, returning an iterable of Event objects from NDJSON. */
    public static Iterable<Event> stream(String bin, List<String> args) throws ZagException {
        List<String> command = buildCommand(bin, args);
        try {
            ProcessBuilder pb = new ProcessBuilder(command);
            pb.redirectErrorStream(false);
            Process process = pb.start();

            BufferedReader reader =
                    new BufferedReader(
                            new InputStreamReader(
                                    process.getInputStream(), StandardCharsets.UTF_8));

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
                                    String line = reader.readLine();
                                    if (line == null) {
                                        done = true;
                                        try {
                                            int exitCode = process.waitFor();
                                            if (exitCode != 0) {
                                                String stderr =
                                                        new String(
                                                                process.getErrorStream()
                                                                        .readAllBytes(),
                                                                StandardCharsets.UTF_8);
                                                throw new RuntimeException(
                                                        new ZagException(
                                                                "zag exited with code " + exitCode,
                                                                exitCode,
                                                                stderr));
                                            }
                                        } catch (InterruptedException e) {
                                            Thread.currentThread().interrupt();
                                        }
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
        } catch (IOException e) {
            throw new ZagException("Failed to start zag: " + e.getMessage(), null, "");
        }
    }

    /** Start a streaming process with piped stdin and stdout. */
    public static StreamingSession startStreamingProcess(String bin, List<String> args)
            throws ZagException {
        List<String> command = buildCommand(bin, args);
        try {
            ProcessBuilder pb = new ProcessBuilder(command);
            pb.redirectErrorStream(false);
            Process process = pb.start();
            return new StreamingSession(process);
        } catch (IOException e) {
            throw new ZagException("Failed to start zag: " + e.getMessage(), null, "");
        }
    }

    /** Run zag interactively with inherited stdio. */
    public static void run(String bin, List<String> args) throws ZagException {
        List<String> command = buildCommand(bin, args);
        try {
            ProcessBuilder pb = new ProcessBuilder(command);
            pb.inheritIO();
            Process process = pb.start();
            int exitCode = process.waitFor();
            if (exitCode != 0) {
                throw new ZagException(
                        "zag exited with code " + exitCode, exitCode, "");
            }
        } catch (ZagException e) {
            throw e;
        } catch (IOException | InterruptedException e) {
            throw new ZagException("Failed to run zag: " + e.getMessage(), null, "");
        }
    }

    private static List<String> buildCommand(String bin, List<String> args) {
        List<String> command = new ArrayList<>(args.size() + 1);
        command.add(bin);
        command.addAll(args);
        return command;
    }
}
