use anyhow::Result;
use clap::{Parser, Subcommand};
use log::debug;
use zag_agent::json_validation;

#[derive(Parser)]
#[command(name = "zag")]
#[command(version, about = "A wrapper for different AI agents")]
pub struct Cli {
    /// Enable debug logging
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Quiet mode - disable all logging except agent output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Verbose mode - show detailed formatted output with icons and status messages
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Skip health check before proxying to remote server
    #[arg(long, global = true)]
    pub no_health_check: bool,

    /// Print an AI-oriented CLI reference and exit
    #[arg(long, global = true)]
    pub help_agent: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Arguments for selecting and configuring an agent (provider, model, etc.)
#[derive(clap::Args, Clone)]
pub(crate) struct AgentArgs {
    /// Provider to use (claude, codex, gemini, copilot, auto)
    #[arg(short = 'p', long)]
    pub(crate) provider: Option<String>,

    /// Model to use (agent-specific, size alias: small/medium/large, or auto)
    #[arg(short, long)]
    pub(crate) model: Option<String>,

    /// Root directory to run the agent in
    #[arg(short, long)]
    pub(crate) root: Option<String>,

    /// Auto-approve all actions (skip permission prompts)
    #[arg(short = 'a', long)]
    pub(crate) auto_approve: bool,

    /// System prompt to configure agent behavior
    #[arg(short, long)]
    pub(crate) system_prompt: Option<String>,

    /// Additional directories to include
    #[arg(long = "add-dir")]
    pub(crate) add_dirs: Vec<String>,

    /// Model parameter size for Ollama (e.g., 0.8b, 2b, 4b, 9b, 27b, 35b, 122b)
    #[arg(long)]
    pub(crate) size: Option<String>,

    /// Show token usage statistics (only applies to JSON output mode)
    #[arg(long)]
    pub(crate) show_usage: bool,

    /// Maximum number of agentic turns
    #[arg(long)]
    pub(crate) max_turns: Option<u32>,

    /// MCP server config for this invocation: JSON string or path to a JSON file (Claude only)
    #[arg(long)]
    pub(crate) mcp_config: Option<String>,

    /// Environment variable to set for the agent subprocess (repeatable, KEY=VALUE format)
    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub(crate) env_vars: Vec<String>,

    /// Attach a file to the prompt (repeatable)
    #[arg(long = "file", value_name = "PATH")]
    pub(crate) files: Vec<String>,
}

/// Arguments for session discovery metadata (name, description, tags)
#[derive(clap::Args, Clone, Default)]
pub(crate) struct SessionMetadataArgs {
    /// Human-readable name for this session (for discovery)
    #[arg(long)]
    pub(crate) name: Option<String>,

    /// Short description of this session's purpose
    #[arg(long)]
    pub(crate) description: Option<String>,

    /// Tags for session discovery (repeatable)
    #[arg(long = "tag")]
    pub(crate) tags: Vec<String>,
}

/// Arguments for session isolation (worktree, sandbox, session ID, JSON output)
#[derive(clap::Args, Clone)]
pub(crate) struct SessionIsolationArgs {
    /// Create a git worktree for this session (optionally specify a name)
    #[arg(short = 'w', long)]
    pub(crate) worktree: Option<Option<String>>,

    /// Run inside a Docker sandbox (optionally specify a name)
    #[arg(long)]
    pub(crate) sandbox: Option<Option<String>>,

    /// Session ID (UUID) to use instead of auto-generating one
    #[arg(long, value_name = "UUID")]
    pub(crate) session: Option<String>,

    /// Request JSON output from the agent
    #[arg(long)]
    pub(crate) json: bool,

    /// JSON schema for structured output (file path or inline JSON string)
    #[arg(long, value_name = "SCHEMA")]
    pub(crate) json_schema: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start an interactive session
    Run {
        /// Initial prompt for the session
        #[arg(conflicts_with = "resume", conflicts_with = "continue_session")]
        prompt: Option<String>,

        /// Resume a specific session
        #[arg(long, value_name = "SESSION_ID", conflicts_with = "continue_session")]
        resume: Option<String>,

        /// Resume the most recent tracked session
        #[arg(long = "continue")]
        continue_session: bool,

        /// Prepend the result of another session as context
        #[arg(long, value_name = "SESSION_ID")]
        context: Option<String>,

        /// Path to a plan file to prepend as context
        #[arg(long, value_name = "PATH")]
        plan: Option<String>,

        #[command(flatten)]
        agent: AgentArgs,

        #[command(flatten)]
        session: SessionIsolationArgs,

        #[command(flatten)]
        metadata: SessionMetadataArgs,
    },
    /// Run non-interactively (print output and exit)
    Exec {
        /// The prompt to send to the agent
        prompt: String,

        /// Resume a specific session
        #[arg(long, value_name = "SESSION_ID")]
        resume: Option<String>,

        /// Resume the most recent tracked session
        #[arg(long = "continue")]
        continue_session: bool,

        /// Output format (text, json, json-pretty, stream-json, native-json)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Input format (text, stream-json) - Claude only
        #[arg(short = 'i', long)]
        input_format: Option<String>,

        /// Re-emit user messages from stdin on stdout (Claude only, requires -i stream-json -o stream-json)
        #[arg(long)]
        replay_user_messages: bool,

        /// Include partial message chunks in streaming output (Claude only, requires -o stream-json)
        #[arg(long)]
        include_partial_messages: bool,

        /// Exit with code 1 if the agent reports failure
        #[arg(long)]
        exit_on_failure: bool,

        /// Prepend the result of another session as context
        #[arg(long, value_name = "SESSION_ID")]
        context: Option<String>,

        /// Path to a plan file to prepend as context
        #[arg(long, value_name = "PATH")]
        plan: Option<String>,

        /// Timeout duration (e.g., 30s, 5m, 1h). Kills the agent if exceeded.
        #[arg(long, value_name = "DURATION")]
        timeout: Option<String>,

        #[command(flatten)]
        agent: AgentArgs,

        #[command(flatten)]
        session: SessionIsolationArgs,

        #[command(flatten)]
        metadata: SessionMetadataArgs,
    },
    /// Review code changes
    Review {
        /// Review staged/unstaged/untracked changes
        #[arg(long)]
        uncommitted: bool,

        /// Review against a base branch
        #[arg(long)]
        base: Option<String>,

        /// Review changes from a specific commit
        #[arg(long)]
        commit: Option<String>,

        /// Optional title for the review summary
        #[arg(long)]
        title: Option<String>,

        /// Additional instructions for the review
        prompt: Option<String>,

        #[command(flatten)]
        agent: AgentArgs,
    },
    /// Generate an implementation plan
    Plan {
        /// What to plan (goal or task description)
        goal: String,

        /// Output path (file or directory; streams to stdout if omitted)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Additional planning instructions
        #[arg(long)]
        instructions: Option<String>,

        #[command(flatten)]
        agent: AgentArgs,
    },
    /// View or set configuration values
    Config {
        /// Config key and value (e.g., "provider claude" or "provider=claude")
        args: Vec<String>,

        /// Root directory for config file resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// List and inspect sessions
    Session {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session store resolution
        #[arg(short, long)]
        root: Option<String>,

        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Show capability declarations for a provider
    Capability {
        /// Output format (json, yaml, toml)
        #[arg(short = 'f', long, default_value = "json")]
        format: String,

        /// Pretty-print output (applies to JSON)
        #[arg(long)]
        pretty: bool,

        /// Provider to use (claude, codex, gemini, copilot)
        #[arg(short = 'p', long)]
        provider: Option<String>,

        /// Root directory for config file resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Discover available providers, models, and capabilities
    Discover {
        /// Filter to a specific provider
        #[arg(short = 'p', long)]
        provider: Option<String>,

        /// Show only model listings
        #[arg(long)]
        models: bool,

        /// Resolve a model alias (e.g. "default", "small", "large")
        #[arg(long, value_name = "MODEL")]
        resolve: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Output format (json, yaml, toml)
        #[arg(short = 'f', long)]
        format: Option<String>,

        /// Pretty-print output (applies to JSON)
        #[arg(long)]
        pretty: bool,

        /// Root directory for config file resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Listen to a session's log events in real-time
    Listen {
        /// Session ID to listen to
        #[arg(conflicts_with_all = ["latest", "active", "ps"])]
        session_id: Option<String>,

        /// Listen to the latest session (most recently created)
        #[arg(long, conflicts_with_all = ["active", "ps"])]
        latest: bool,

        /// Listen to the active session (most recently written-to log file)
        #[arg(long, conflicts_with_all = ["latest", "ps"])]
        active: bool,

        /// Listen to the session of a process by OS PID or zag process UUID
        #[arg(long, value_name = "PID", conflicts_with_all = ["session_id", "latest", "active"])]
        ps: Option<String>,

        /// Output as JSON (NDJSON)
        #[arg(long)]
        json: bool,

        /// Output as plain text (default)
        #[arg(long)]
        text: bool,

        /// Enable rich text output (ANSI colors, bold, dim, italic)
        #[arg(long, visible_alias = "colors")]
        rich_text: bool,

        /// Show thinking/reasoning content
        #[arg(long)]
        show_thinking: bool,

        /// Show timestamps for each event
        #[arg(long)]
        timestamps: bool,

        /// Filter by event type (repeatable: session_started, user_message, assistant_message, reasoning, tool_call, tool_result, permission, session_ended)
        #[arg(long = "filter", value_name = "EVENT_TYPE")]
        filters: Vec<String>,

        /// Root directory for session log resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Show manual pages for commands
    Man {
        /// Command to show help for (run, exec, review, config, session, capability, listen, input, man, skills, mcp, ps)
        command: Option<String>,
    },
    /// Manage provider-agnostic skills stored in ~/.zag/skills/
    Skills {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        #[command(subcommand)]
        command: SkillsCommand,
    },
    /// Manage MCP (Model Context Protocol) servers across providers
    Mcp {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for project-scoped resolution
        #[arg(short, long)]
        root: Option<String>,

        #[command(subcommand)]
        command: McpCommand,
    },
    /// List, inspect, and kill agent processes started by zag
    Ps {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        #[command(subcommand)]
        command: Option<zag_orch::ps::PsCommand>,
    },
    /// Search through session logs
    Search {
        /// Text to search for (literal substring by default; use --regex for patterns)
        query: Option<String>,

        /// Treat the query as a regular expression
        #[arg(long)]
        regex: bool,

        /// Case-sensitive search (default is case-insensitive)
        #[arg(long)]
        case_sensitive: bool,

        /// Filter by provider (claude, codex, gemini, copilot, ollama)
        #[arg(short = 'p', long)]
        provider: Option<String>,

        /// Filter by message role (user, assistant)
        #[arg(long)]
        role: Option<String>,

        /// Filter by tool name (case-insensitive substring)
        #[arg(long)]
        tool: Option<String>,

        /// Filter by tool kind
        #[arg(long, value_enum)]
        tool_kind: Option<ToolKindArg>,

        /// Show only events at or after this time (ISO 8601 or relative: 1h, 2d, 3w, 1m)
        #[arg(long)]
        from: Option<String>,

        /// Show only events at or before this time
        #[arg(long)]
        to: Option<String>,

        /// Restrict search to a specific session ID (prefix match)
        #[arg(long, value_name = "SESSION_ID")]
        session: Option<String>,

        /// Filter by session tag (exact match, case-insensitive)
        #[arg(long)]
        tag: Option<String>,

        /// Search all sessions across all projects (default: current project and sub-projects)
        #[arg(long)]
        global: bool,

        /// Output results as NDJSON (one JSON object per match)
        #[arg(long, short = 'j')]
        json: bool,

        /// Output only the count of matches
        #[arg(long, short = 'c')]
        count: bool,

        /// Maximum number of matches to return
        #[arg(long, short = 'n')]
        limit: Option<usize>,

        /// Root directory for project scope resolution (overrides cwd)
        #[arg(long)]
        root: Option<String>,
    },
    /// Show identity of the current zag session (for agent introspection)
    Whoami {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Send a user message to a single running or resumable session
    Input {
        /// Message to send (reads from stdin if omitted and not --stream)
        message: Option<String>,

        /// Target a specific session by ID
        #[arg(long, value_name = "SESSION_ID", conflicts_with_all = ["latest", "active", "ps", "input_name"])]
        session: Option<String>,

        /// Send to the most recently created session
        #[arg(long, conflicts_with_all = ["active", "ps", "input_name"])]
        latest: bool,

        /// Send to the most recently active session
        #[arg(long, conflicts_with_all = ["latest", "ps", "input_name"])]
        active: bool,

        /// Send to a session by PID or zag process UUID
        #[arg(long, value_name = "PID", conflicts_with_all = ["session", "latest", "active", "input_name"])]
        ps: Option<String>,

        /// Target a session by name
        #[arg(long = "name", id = "input_name", value_name = "NAME", conflicts_with_all = ["session", "latest", "active", "ps"])]
        input_name: Option<String>,

        /// Search across all projects when auto-resolving the session
        #[arg(long)]
        global: bool,

        /// Stream multiple messages from stdin (Claude only)
        #[arg(long)]
        stream: bool,

        /// Output format (text, json, stream-json)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,

        /// Send without agent-to-agent envelope (skip sender metadata wrapping)
        #[arg(long)]
        raw: bool,

        /// Attach a file to the message (repeatable)
        #[arg(long = "file", value_name = "PATH")]
        files: Vec<String>,
    },
    /// Export session environment variables for nested agent invocations
    Env {
        /// Session ID (defaults to latest)
        session_id: Option<String>,

        /// Output as shell export statements (for eval)
        #[arg(long)]
        shell: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Gather results from multiple sessions
    Collect {
        /// Session IDs to collect results from
        session_ids: Vec<String>,

        /// Collect results from all sessions with this tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Show session status (running, idle, completed, failed, dead, unknown)
    Status {
        /// Session ID to check
        session_id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Launch an agent session in the background, print session ID, and exit
    Spawn {
        /// The prompt to send to the agent (optional with --interactive)
        prompt: Option<String>,

        /// Path to a plan file to prepend as context
        #[arg(long, value_name = "PATH")]
        plan: Option<String>,

        #[command(flatten)]
        agent: AgentArgs,

        #[command(flatten)]
        metadata: SessionMetadataArgs,

        /// Output as JSON (includes session_id, pid, log_path)
        #[arg(long)]
        json: bool,

        /// Wait for these sessions to complete before starting (repeatable)
        #[arg(long = "depends-on", value_name = "SESSION_ID")]
        depends_on: Vec<String>,

        /// Auto-inject dependency session results as context
        #[arg(long)]
        inject_context: bool,

        /// Timeout duration (e.g., 30s, 5m, 1h). Kills the agent if exceeded.
        #[arg(long, value_name = "DURATION")]
        timeout: Option<String>,

        /// Spawn a long-lived interactive session (FIFO-based, usable with zag input)
        #[arg(long, short = 'I')]
        interactive: bool,
    },
    /// Block until one or more sessions complete
    Wait {
        /// Session IDs to wait for
        session_ids: Vec<String>,

        /// Wait for all sessions with this tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Wait for the latest session
        #[arg(long)]
        latest: bool,

        /// Timeout duration (e.g., 30s, 5m, 1h)
        #[arg(long, value_name = "DURATION")]
        timeout: Option<String>,

        /// Exit on first completed session (instead of waiting for all)
        #[arg(long)]
        any: bool,

        /// Output as JSON (NDJSON, one result per line)
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Chain results from completed sessions into a new agent session
    Pipe {
        /// Session IDs to pipe results from
        session_ids: Vec<String>,

        /// Pipe results from all sessions with this tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// The prompt to send with the piped context (after --)
        #[arg(last = true)]
        prompt: String,

        #[command(flatten)]
        agent: AgentArgs,

        /// Output format (text, json, json-pretty)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Request JSON output from the agent
        #[arg(long)]
        json: bool,
    },
    /// Query structured events from session logs
    Events {
        /// Session ID to query events from
        session_id: String,

        /// Filter by event type (session_started, user_message, assistant_message, tool_call, etc.)
        #[arg(long = "type", value_name = "EVENT_TYPE")]
        event_type: Option<String>,

        /// Show only the last N events
        #[arg(long)]
        last: Option<usize>,

        /// Show events after this sequence number (for pagination/polling)
        #[arg(long, value_name = "SEQ")]
        after_seq: Option<u64>,

        /// Show events before this sequence number
        #[arg(long, value_name = "SEQ")]
        before_seq: Option<u64>,

        /// Output only the count of matching events
        #[arg(long)]
        count: bool,

        /// Output as NDJSON
        #[arg(long)]
        json: bool,

        /// Root directory for session log resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Gracefully cancel one or more running sessions
    Cancel {
        /// Session IDs to cancel
        session_ids: Vec<String>,

        /// Cancel all sessions with this tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Reason for cancellation
        #[arg(long)]
        reason: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Show a log-based summary of one or more sessions
    Summary {
        /// Session IDs to summarize
        session_ids: Vec<String>,

        /// Summarize all sessions with this tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Show detailed statistics
        #[arg(long)]
        stats: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Watch session logs and execute a command on matching events
    Watch {
        /// Session ID to watch
        #[arg(conflicts_with_all = ["latest", "watch_tag"])]
        session_id: Option<String>,

        /// Watch sessions with this tag
        #[arg(long = "tag", id = "watch_tag", value_name = "TAG", conflicts_with_all = ["latest"])]
        tag: Option<String>,

        /// Watch the latest session
        #[arg(long)]
        latest: bool,

        /// Event type to watch for (e.g., session_ended, tool_call)
        #[arg(long = "on", value_name = "EVENT_TYPE")]
        on_event: String,

        /// Filter expression (key=value pairs, comma-separated)
        #[arg(long = "filter")]
        filter_expr: Option<String>,

        /// Exit after the first matching event
        #[arg(long)]
        once: bool,

        /// Output matching events as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session log resolution
        #[arg(short, long)]
        root: Option<String>,

        /// Command to execute (after --)
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Subscribe to a multiplexed event stream from all active sessions
    Subscribe {
        /// Filter by session tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Filter by event type
        #[arg(long = "filter", value_name = "EVENT_TYPE")]
        event_type: Option<String>,

        /// Subscribe across all projects
        #[arg(long)]
        global: bool,

        /// Output as NDJSON (default)
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Send a message to all sessions in the current project (optionally filtered by tag)
    Broadcast {
        /// Message to send (reads from stdin if omitted)
        message: Option<String>,

        /// Filter target sessions by tag (optional; sends to all project sessions if omitted)
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Search across all projects
        #[arg(long)]
        global: bool,

        /// Output format (text, json, json-pretty)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,

        /// Send without agent-to-agent envelope (skip sender metadata wrapping)
        #[arg(long)]
        raw: bool,
    },
    /// Append a custom event to a session's log
    Log {
        /// Event message
        message: String,

        /// Target session ID (defaults to ZAG_SESSION_ID env var)
        #[arg(long, value_name = "SESSION_ID")]
        session: Option<String>,

        /// Event level (info, warn, error, debug)
        #[arg(long, default_value = "info")]
        level: String,

        /// Structured JSON data to attach to the event
        #[arg(long, value_name = "JSON")]
        data: Option<String>,

        /// Root directory for session log resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Extract the final result text from a session
    Output {
        /// Session ID (defaults to latest)
        session_id: Option<String>,

        /// Get result from the latest session
        #[arg(long, conflicts_with_all = ["session_id", "output_name"])]
        latest: bool,

        /// Get result from a session by name
        #[arg(long = "name", id = "output_name", value_name = "NAME", conflicts_with_all = ["session_id", "latest"])]
        output_name: Option<String>,

        /// Get results from sessions with this tag
        #[arg(long, value_name = "TAG", conflicts_with_all = ["session_id", "latest", "output_name"])]
        tag: Option<String>,

        /// Output as JSON (includes session_id and metadata)
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Re-run a failed session with the same configuration
    Retry {
        /// Session IDs to retry
        session_ids: Vec<String>,

        /// Retry all sessions with this tag
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,

        /// Only retry sessions that failed (skip completed ones)
        #[arg(long)]
        failed: bool,

        /// Override the model for retried sessions
        #[arg(short, long)]
        model: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Clean up old session data, logs, and process entries
    Gc {
        /// Actually delete (default is dry-run)
        #[arg(long)]
        force: bool,

        /// Only clean data older than this threshold (e.g. 7d, 30d, 24h)
        #[arg(long, default_value = "7d")]
        older_than: String,

        /// Keep session log files (only clean process/marker entries)
        #[arg(long)]
        keep_logs: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Root directory for session resolution
        #[arg(short, long)]
        root: Option<String>,
    },
    /// Start the zag HTTPS/WebSocket server for remote access
    Serve {
        /// Bind address (default: 0.0.0.0, or from ~/.zag/serve.toml)
        #[arg(long)]
        host: Option<String>,

        /// Port to listen on (default: 2100, or from ~/.zag/serve.toml)
        #[arg(long)]
        port: Option<u16>,

        /// Authentication token (or use ZAG_SERVE_TOKEN env var)
        #[arg(long)]
        token: Option<String>,

        /// Generate a new token, save it, and start the server
        #[arg(long)]
        generate_token: bool,

        /// TLS certificate file path (PEM format); overrides auto-generated certificate
        #[arg(long, value_name = "PATH")]
        tls_cert: Option<String>,

        /// TLS private key file path (PEM format); overrides auto-generated certificate
        #[arg(long, value_name = "PATH")]
        tls_key: Option<String>,

        /// Force all connected users' agent sessions to run inside a Docker sandbox
        #[arg(long)]
        force_sandbox: bool,
    },
    /// Connect to a remote zag server (all subsequent commands proxy through it)
    Connect {
        /// Server URL (e.g., https://home.local:2100)
        url: String,

        /// Authentication token (or use ZAG_CONNECT_TOKEN env var)
        #[arg(long, conflicts_with = "username")]
        token: Option<String>,

        /// Username for user-account authentication
        #[arg(long, short = 'u', conflicts_with = "token")]
        username: Option<String>,

        /// Password for user-account authentication (prompted if not provided)
        #[arg(long, requires = "username")]
        password: Option<String>,
    },
    /// Disconnect from the remote zag server
    Disconnect,
    /// Manage user accounts on the server
    User {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        #[command(subcommand)]
        command: UserCommand,
    },
    /// Internal: relay for interactive sessions (FIFO-based streaming)
    #[command(hide = true)]
    Relay {
        /// Session ID
        #[arg(long)]
        session: String,

        #[command(flatten)]
        agent: AgentArgs,

        /// Optional initial prompt
        prompt: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum SessionCommand {
    /// List all sessions
    List {
        /// Filter by provider
        #[arg(short = 'p', long)]
        provider: Option<String>,
        /// Show only the N most recent sessions
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// List sessions across all projects
        #[arg(long)]
        global: bool,
        /// Filter by session name (substring match, case-insensitive)
        #[arg(long)]
        name: Option<String>,
        /// Filter by tag (exact match, case-insensitive)
        #[arg(long)]
        tag: Option<String>,
        /// Show only sessions spawned by this parent session ID
        #[arg(long)]
        parent: Option<String>,
    },
    /// Show details of a specific session
    Show {
        /// Session ID (wrapper or provider-native)
        id: String,
    },
    /// Import historical provider logs into the session store
    Import,
    /// Delete a session from the store
    Delete {
        /// Session ID to delete
        id: String,
    },
    /// Update session metadata (name, description, tags)
    Update {
        /// Session ID to update
        id: String,
        /// Set session name
        #[arg(long)]
        name: Option<String>,
        /// Set session description
        #[arg(long)]
        description: Option<String>,
        /// Add tags (repeatable)
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// Clear all existing tags before adding new ones
        #[arg(long)]
        clear_tags: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum SkillsCommand {
    /// List all available skills
    List,
    /// Show details of a specific skill
    Show {
        /// Skill name to show
        name: String,
    },
    /// Create a new skill skeleton
    Add {
        /// Skill name (directory name)
        name: String,
        /// Short description of what the skill does
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove a skill and its provider symlinks
    Remove {
        /// Skill name to remove
        name: String,
    },
    /// Sync skills to all provider-specific locations
    Sync {
        /// Only sync for this provider (claude, gemini, copilot, codex)
        #[arg(short = 'p', long)]
        provider: Option<String>,
    },
    /// Import existing skills from a provider's native skill directory
    Import {
        /// Provider to import from (default: claude)
        #[arg(long, default_value = "claude")]
        from: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum McpCommand {
    /// List all configured MCP servers
    List,
    /// Show details of a specific MCP server
    Show {
        /// Server name to show
        name: String,
    },
    /// Add a new MCP server
    Add {
        /// Server name (used as filename and provider key)
        name: String,
        /// Transport type: stdio or http
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// Command to start the server (stdio transport)
        #[arg(long)]
        command: Option<String>,
        /// Arguments for the command
        #[arg(long, num_args = 1..)]
        args: Vec<String>,
        /// URL endpoint (http transport)
        #[arg(long)]
        url: Option<String>,
        /// Environment variables (KEY=VALUE pairs, repeatable)
        #[arg(long, value_name = "KEY=VALUE")]
        env: Vec<String>,
        /// Short description
        #[arg(long)]
        description: Option<String>,
        /// Store in global directory (~/.zag/mcp/) instead of project-scoped
        #[arg(long)]
        global: bool,
    },
    /// Remove an MCP server and clean up provider configs
    Remove {
        /// Server name to remove
        name: String,
    },
    /// Sync MCP servers to all provider-specific configs
    Sync {
        /// Only sync for this provider (claude, gemini, copilot, codex)
        #[arg(short = 'p', long)]
        provider: Option<String>,
    },
    /// Import MCP servers from a provider's native config
    Import {
        /// Provider to import from (default: claude)
        #[arg(long, default_value = "claude")]
        from: String,
    },
}

/// Bridge enum for `ToolKind` that derives `clap::ValueEnum` (kept in the binary crate
/// so `zag-agent` does not need a clap dependency).
#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum ToolKindArg {
    Shell,
    FileRead,
    FileWrite,
    FileEdit,
    Search,
    SubAgent,
    Web,
    Notebook,
    Other,
}

impl From<ToolKindArg> for zag_agent::session_log::ToolKind {
    fn from(a: ToolKindArg) -> Self {
        match a {
            ToolKindArg::Shell => zag_agent::session_log::ToolKind::Shell,
            ToolKindArg::FileRead => zag_agent::session_log::ToolKind::FileRead,
            ToolKindArg::FileWrite => zag_agent::session_log::ToolKind::FileWrite,
            ToolKindArg::FileEdit => zag_agent::session_log::ToolKind::FileEdit,
            ToolKindArg::Search => zag_agent::session_log::ToolKind::Search,
            ToolKindArg::SubAgent => zag_agent::session_log::ToolKind::SubAgent,
            ToolKindArg::Web => zag_agent::session_log::ToolKind::Web,
            ToolKindArg::Notebook => zag_agent::session_log::ToolKind::Notebook,
            ToolKindArg::Other => zag_agent::session_log::ToolKind::Other,
        }
    }
}

/// Extract AgentArgs from a command, if it has them.
pub(crate) fn command_agent_args(cmd: &Commands) -> Option<&AgentArgs> {
    match cmd {
        Commands::Run { agent, .. } => Some(agent),
        Commands::Exec { agent, .. } => Some(agent),
        Commands::Review { agent, .. } => Some(agent),
        Commands::Plan { agent, .. } => Some(agent),
        Commands::Spawn { agent, .. } => Some(agent),
        Commands::Pipe { agent, .. } => Some(agent),
        _ => None,
    }
}

/// Extract SessionIsolationArgs from a command, if it has them.
pub(crate) fn command_session_args(cmd: &Commands) -> Option<&SessionIsolationArgs> {
    match cmd {
        Commands::Run { session, .. } => Some(session),
        Commands::Exec { session, .. } => Some(session),
        _ => None,
    }
}

/// Extract SessionMetadataArgs from a command, if it has them.
pub(crate) fn command_metadata_args(cmd: &Commands) -> Option<&SessionMetadataArgs> {
    match cmd {
        Commands::Run { metadata, .. } => Some(metadata),
        Commands::Exec { metadata, .. } => Some(metadata),
        Commands::Spawn { metadata, .. } => Some(metadata),
        _ => None,
    }
}

/// Parse and validate a JSON schema string, returning the parsed value.
pub(crate) fn parse_json_schema(schema_str: &str) -> Result<serde_json::Value> {
    let schema_json = if std::path::Path::new(schema_str).exists() {
        let content = std::fs::read_to_string(schema_str)
            .map_err(|e| anyhow::anyhow!("Failed to read JSON schema file '{schema_str}': {e}"))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in schema file '{schema_str}': {e}"))?
    } else {
        serde_json::from_str::<serde_json::Value>(schema_str)
            .map_err(|e| anyhow::anyhow!("Invalid JSON schema: {e}"))?
    };
    json_validation::validate_schema(&schema_json).map_err(|e| anyhow::anyhow!("{e}"))?;
    debug!(
        "JSON schema loaded: {} bytes",
        serde_json::to_string(&schema_json)
            .unwrap_or_default()
            .len()
    );
    Ok(schema_json)
}

#[derive(Subcommand)]
pub(crate) enum UserCommand {
    /// Add a new user account
    Add {
        /// Username
        #[arg(long, short = 'u')]
        username: String,

        /// Home directory (the user will be locked to this directory)
        #[arg(long)]
        home_dir: String,

        /// Password (prompted interactively if not provided)
        #[arg(long)]
        password: Option<String>,
    },
    /// Remove a user account
    Remove {
        /// Username to remove
        username: String,
    },
    /// List all user accounts
    List,
    /// Change a user's password
    Passwd {
        /// Username
        username: String,

        /// New password (prompted interactively if not provided)
        #[arg(long)]
        password: Option<String>,
    },
}
