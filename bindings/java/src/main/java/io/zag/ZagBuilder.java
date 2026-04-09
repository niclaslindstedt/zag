package io.zag;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.ArrayList;
import java.util.List;

/**
 * Fluent builder for configuring and running zag agent sessions.
 *
 * <pre>{@code
 * AgentOutput output = new ZagBuilder()
 *     .provider("claude")
 *     .model("sonnet")
 *     .autoApprove()
 *     .exec("write a hello world program");
 *
 * System.out.println(output.result());
 * }</pre>
 */
public class ZagBuilder {

    private static final ObjectMapper MAPPER = ZagProcess.MAPPER;

    private String bin = ZagProcess.defaultBin();
    private String provider;
    private String model;
    private String systemPrompt;
    private String root;
    private boolean autoApprove;
    private final List<String> addDirs = new ArrayList<>();
    private final List<String> files = new ArrayList<>();
    private final List<String> envVars = new ArrayList<>();
    private boolean json;
    private Object jsonSchema;
    private boolean jsonStream;
    private Object worktree;   // Boolean.TRUE or String
    private Object sandbox;    // Boolean.TRUE or String
    private boolean verbose;
    private boolean quiet;
    private boolean debug;
    private String sessionId;
    private String outputFormat;
    private String inputFormat;
    private boolean replayUserMessages;
    private boolean includePartialMessages;
    private Integer maxTurns;
    private String timeout;
    private String mcpConfig;
    private boolean showUsage;
    private String size;

    // -- Configuration methods -----------------------------------------------

    /** Override the zag binary path (default: {@code ZAG_BIN} env or {@code "zag"}). */
    public ZagBuilder bin(String path) { this.bin = path; return this; }

    /** Set the provider (e.g., "claude", "codex", "gemini", "copilot", "ollama"). */
    public ZagBuilder provider(String p) { this.provider = p; return this; }

    /** Set the model (e.g., "sonnet", "opus", "small", "large"). */
    public ZagBuilder model(String m) { this.model = m; return this; }

    /** Set a system prompt to configure agent behavior. */
    public ZagBuilder systemPrompt(String p) { this.systemPrompt = p; return this; }

    /** Set the root directory for the agent to operate in. */
    public ZagBuilder root(String r) { this.root = r; return this; }

    /** Enable auto-approve mode (skip permission prompts). */
    public ZagBuilder autoApprove() { this.autoApprove = true; return this; }

    /** Enable or disable auto-approve mode. */
    public ZagBuilder autoApprove(boolean a) { this.autoApprove = a; return this; }

    /** Add an additional directory for the agent to include. */
    public ZagBuilder addDir(String d) { this.addDirs.add(d); return this; }

    /** Attach a file to the prompt (chainable). */
    public ZagBuilder file(String path) { this.files.add(path); return this; }

    /** Add an environment variable for the agent subprocess. */
    public ZagBuilder env(String key, String value) { this.envVars.add(key + "=" + value); return this; }

    /** Request JSON output from the agent. */
    public ZagBuilder json() { this.json = true; return this; }

    /** Set a JSON schema for structured output validation. Implies {@link #json()}. */
    public ZagBuilder jsonSchema(Object s) { this.jsonSchema = s; this.json = true; return this; }

    /** Enable streaming JSON output (NDJSON format). */
    public ZagBuilder jsonStream() { this.jsonStream = true; return this; }

    /** Enable worktree mode. */
    public ZagBuilder worktree() { this.worktree = Boolean.TRUE; return this; }

    /** Enable worktree mode with a name. */
    public ZagBuilder worktree(String name) { this.worktree = name; return this; }

    /** Enable sandbox mode. */
    public ZagBuilder sandbox() { this.sandbox = Boolean.TRUE; return this; }

    /** Enable sandbox mode with a name. */
    public ZagBuilder sandbox(String name) { this.sandbox = name; return this; }

    /** Enable verbose output. */
    public ZagBuilder verbose() { this.verbose = true; return this; }

    /** Enable or disable verbose output. */
    public ZagBuilder verbose(boolean v) { this.verbose = v; return this; }

    /** Enable quiet mode. */
    public ZagBuilder quiet() { this.quiet = true; return this; }

    /** Enable or disable quiet mode. */
    public ZagBuilder quiet(boolean q) { this.quiet = q; return this; }

    /** Enable debug logging. */
    public ZagBuilder debug() { this.debug = true; return this; }

    /** Enable or disable debug logging. */
    public ZagBuilder debug(boolean d) { this.debug = d; return this; }

    /** Pre-set a session ID (UUID). */
    public ZagBuilder sessionId(String id) { this.sessionId = id; return this; }

    /** Set the output format (e.g., "text", "json", "json-pretty", "stream-json"). */
    public ZagBuilder outputFormat(String f) { this.outputFormat = f; return this; }

    /** Set the input format (Claude only, e.g., "text", "stream-json"). */
    public ZagBuilder inputFormat(String f) { this.inputFormat = f; return this; }

    /** Re-emit user messages from stdin on stdout (Claude only). */
    public ZagBuilder replayUserMessages() { this.replayUserMessages = true; return this; }

    /** Enable or disable replay of user messages (Claude only). */
    public ZagBuilder replayUserMessages(boolean r) { this.replayUserMessages = r; return this; }

    /** Include partial message chunks in streaming output (Claude only). */
    public ZagBuilder includePartialMessages() { this.includePartialMessages = true; return this; }

    /** Enable or disable partial message chunks (Claude only). */
    public ZagBuilder includePartialMessages(boolean i) { this.includePartialMessages = i; return this; }

    /** Set the maximum number of agentic turns. */
    public ZagBuilder maxTurns(int n) { this.maxTurns = n; return this; }

    /** Set a timeout duration (e.g., "30s", "5m", "1h"). Kills the agent if exceeded. */
    public ZagBuilder timeout(String t) { this.timeout = t; return this; }

    /** Set MCP server config for this invocation: JSON string or file path (Claude only). */
    public ZagBuilder mcpConfig(String c) { this.mcpConfig = c; return this; }

    /** Show token usage statistics (only applies to JSON output mode). */
    public ZagBuilder showUsage() { this.showUsage = true; return this; }

    /** Enable or disable token usage statistics. */
    public ZagBuilder showUsage(boolean s) { this.showUsage = s; return this; }

    /** Set the Ollama model parameter size (e.g., "2b", "9b", "35b"). */
    public ZagBuilder size(String s) { this.size = s; return this; }

    // -- Arg building --------------------------------------------------------

    /** Build the shared CLI flags (provider, model, session isolation, etc.). */
    List<String> buildGlobalArgs() {
        List<String> args = new ArrayList<>();
        if (provider != null) { args.add("-p"); args.add(provider); }
        if (model != null) { args.add("--model"); args.add(model); }
        if (systemPrompt != null) { args.add("--system-prompt"); args.add(systemPrompt); }
        if (root != null) { args.add("--root"); args.add(root); }
        if (autoApprove) args.add("--auto-approve");
        for (String d : addDirs) { args.add("--add-dir"); args.add(d); }
        for (String f : files) { args.add("--file"); args.add(f); }
        for (String e : envVars) { args.add("--env"); args.add(e); }
        if (worktree instanceof Boolean) {
            args.add("-w");
        } else if (worktree instanceof String wt) {
            args.add("-w"); args.add(wt);
        }
        if (sandbox instanceof Boolean) {
            args.add("--sandbox");
        } else if (sandbox instanceof String sb) {
            args.add("--sandbox"); args.add(sb);
        }
        if (verbose) args.add("--verbose");
        if (quiet) args.add("--quiet");
        if (debug) args.add("--debug");
        if (sessionId != null) { args.add("--session"); args.add(sessionId); }
        if (maxTurns != null) { args.add("--max-turns"); args.add(String.valueOf(maxTurns)); }
        if (mcpConfig != null) { args.add("--mcp-config"); args.add(mcpConfig); }
        if (showUsage) args.add("--show-usage");
        if (size != null) { args.add("--size"); args.add(size); }
        return args;
    }

    /** Build CLI args for exec mode. */
    List<String> buildExecArgs(String prompt, boolean streaming) {
        List<String> args = new ArrayList<>();
        args.add("exec");
        args.addAll(buildGlobalArgs());
        if (json) args.add("--json");
        if (jsonSchema != null) {
            args.add("--json-schema");
            try {
                args.add(MAPPER.writeValueAsString(jsonSchema));
            } catch (Exception e) {
                throw new IllegalArgumentException("Failed to serialize JSON schema", e);
            }
        }
        if (jsonStream || streaming) args.add("--json-stream");
        if (outputFormat != null) { args.add("-o"); args.add(outputFormat); }
        if (inputFormat != null) { args.add("-i"); args.add(inputFormat); }
        if (replayUserMessages) args.add("--replay-user-messages");
        if (includePartialMessages) args.add("--include-partial-messages");
        if (timeout != null) { args.add("--timeout"); args.add(timeout); }
        // Default to json output for structured parsing
        if (!streaming && outputFormat == null && !jsonStream) {
            args.add("-o"); args.add("json");
        }
        args.add(prompt);
        return args;
    }

    // -- Version requirements -------------------------------------------------

    private List<VersionCheck.Requirement> versionRequirements() {
        return List.of(
            new VersionCheck.Requirement("env()", "0.6.0", !envVars.isEmpty()),
            new VersionCheck.Requirement("mcpConfig()", "0.6.0", mcpConfig != null)
        );
    }

    // -- Terminal methods ----------------------------------------------------

    /**
     * Run the agent non-interactively and return structured output.
     *
     * <pre>{@code
     * AgentOutput output = new ZagBuilder()
     *     .provider("claude")
     *     .exec("say hello");
     * System.out.println(output.result());
     * }</pre>
     */
    public AgentOutput exec(String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = buildExecArgs(prompt, false);
        return ZagProcess.exec(bin, args);
    }

    /**
     * Run the agent in streaming mode, yielding events as they arrive.
     *
     * <pre>{@code
     * for (Event event : new ZagBuilder()
     *         .provider("claude")
     *         .stream("analyze this code")) {
     *     System.out.println(event.type());
     * }
     * }</pre>
     */
    public Iterable<Event> stream(String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = buildExecArgs(prompt, true);
        return ZagProcess.stream(bin, args);
    }

    /**
     * Run the agent with streaming input and output (Claude only).
     *
     * <p>Returns a StreamingSession with piped stdin for sending NDJSON messages
     * and an iterator for reading events. Automatically enables
     * {@code --input-format stream-json}, {@code --replay-user-messages}, and
     * {@code -o stream-json}.
     *
     * <pre>{@code
     * try (StreamingSession session = new ZagBuilder()
     *         .provider("claude")
     *         .execStreaming("initial prompt")) {
     *     session.send("{\"type\":\"user_message\",\"content\":\"hello\"}");
     *     for (Event event : session.events()) {
     *         System.out.println(event.type());
     *     }
     *     session.await();
     * }
     * }</pre>
     */
    public StreamingSession execStreaming(String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = new ArrayList<>();
        args.add("exec");
        args.addAll(buildGlobalArgs());
        args.add("-i"); args.add("stream-json");
        args.add("-o"); args.add("stream-json");
        args.add("--replay-user-messages");
        if (includePartialMessages) args.add("--include-partial-messages");
        if (outputFormat != null) { args.add("-o"); args.add(outputFormat); }
        args.add(prompt);
        return ZagProcess.startStreamingProcess(bin, args);
    }

    /** Start an interactive agent session (inherits stdio). */
    public void run(String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = new ArrayList<>();
        args.add("run");
        args.addAll(buildGlobalArgs());
        if (json) args.add("--json");
        if (jsonSchema != null) {
            args.add("--json-schema");
            try {
                args.add(MAPPER.writeValueAsString(jsonSchema));
            } catch (Exception e) {
                throw new IllegalArgumentException("Failed to serialize JSON schema", e);
            }
        }
        if (prompt != null) args.add(prompt);
        ZagProcess.run(bin, args);
    }

    /** Start an interactive agent session without a prompt. */
    public void run() throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        run(null);
    }

    /** Resume a previous session by ID. */
    public void resume(String sessionId) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = new ArrayList<>();
        args.add("run");
        args.addAll(buildGlobalArgs());
        args.add("--resume");
        args.add(sessionId);
        ZagProcess.run(bin, args);
    }

    /** Resume the most recent session. */
    public void continueLast() throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = new ArrayList<>();
        args.add("run");
        args.addAll(buildGlobalArgs());
        args.add("--continue");
        ZagProcess.run(bin, args);
    }

    /** Resume a previous session non-interactively with a follow-up prompt. */
    public AgentOutput execResume(String sessionId, String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = buildExecArgs(prompt, false);
        args.add(args.size() - 1, "--resume");
        args.add(args.size() - 1, sessionId);
        return ZagProcess.exec(bin, args);
    }

    /** Resume the most recent session non-interactively with a follow-up prompt. */
    public AgentOutput execContinue(String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = buildExecArgs(prompt, false);
        args.add(args.size() - 1, "--continue");
        return ZagProcess.exec(bin, args);
    }

    /** Resume a previous session in streaming mode with a follow-up prompt. */
    public Iterable<Event> streamResume(String sessionId, String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = buildExecArgs(prompt, true);
        args.add(args.size() - 1, "--resume");
        args.add(args.size() - 1, sessionId);
        return ZagProcess.stream(bin, args);
    }

    /** Resume the most recent session in streaming mode with a follow-up prompt. */
    public Iterable<Event> streamContinue(String prompt) throws ZagException {
        VersionCheck.check(bin, versionRequirements());
        List<String> args = buildExecArgs(prompt, true);
        args.add(args.size() - 1, "--continue");
        return ZagProcess.stream(bin, args);
    }
}
