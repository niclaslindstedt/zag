use anyhow::Result;
use clap::{Parser, Subcommand};
use log::debug;
use zag::json_validation;

#[derive(Parser)]
#[command(name = "zag")]
#[command(about = "A wrapper for different AI agents")]
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

    /// Stream JSON events (NDJSON format) — sets output format to stream-json
    #[arg(long)]
    pub(crate) json_stream: bool,
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

        #[command(flatten)]
        agent: AgentArgs,

        #[command(flatten)]
        session: SessionIsolationArgs,
    },
    /// Run non-interactively (print output and exit)
    Exec {
        /// The prompt to send to the agent
        prompt: String,

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

        #[command(flatten)]
        agent: AgentArgs,

        #[command(flatten)]
        session: SessionIsolationArgs,
    },
    /// Review code changes (uses Codex under the hood)
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
        command: Option<crate::ps::PsCommand>,
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
    /// Send a user message to a running or resumable session
    Input {
        /// Message to send (reads from stdin if omitted and not --stream)
        message: Option<String>,

        /// Target a specific session by ID
        #[arg(long, value_name = "SESSION_ID", conflicts_with_all = ["latest", "active", "ps"])]
        session: Option<String>,

        /// Send to the most recently created session
        #[arg(long, conflicts_with_all = ["active", "ps"])]
        latest: bool,

        /// Send to the most recently active session
        #[arg(long, conflicts_with_all = ["latest", "ps"])]
        active: bool,

        /// Send to a session by PID or zag process UUID
        #[arg(long, value_name = "PID", conflicts_with_all = ["session", "latest", "active"])]
        ps: Option<String>,

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
/// so `zag-lib` does not need a clap dependency).
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

impl From<ToolKindArg> for zag::session_log::ToolKind {
    fn from(a: ToolKindArg) -> Self {
        match a {
            ToolKindArg::Shell => zag::session_log::ToolKind::Shell,
            ToolKindArg::FileRead => zag::session_log::ToolKind::FileRead,
            ToolKindArg::FileWrite => zag::session_log::ToolKind::FileWrite,
            ToolKindArg::FileEdit => zag::session_log::ToolKind::FileEdit,
            ToolKindArg::Search => zag::session_log::ToolKind::Search,
            ToolKindArg::SubAgent => zag::session_log::ToolKind::SubAgent,
            ToolKindArg::Web => zag::session_log::ToolKind::Web,
            ToolKindArg::Notebook => zag::session_log::ToolKind::Notebook,
            ToolKindArg::Other => zag::session_log::ToolKind::Other,
        }
    }
}

/// Extract AgentArgs from a command, if it has them.
pub(crate) fn command_agent_args(cmd: &Commands) -> Option<&AgentArgs> {
    match cmd {
        Commands::Run { agent, .. } => Some(agent),
        Commands::Exec { agent, .. } => Some(agent),
        Commands::Review { agent, .. } => Some(agent),
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

/// Parse and validate a JSON schema string, returning the parsed value.
pub(crate) fn parse_json_schema(schema_str: &str) -> Result<serde_json::Value> {
    let schema_json = if std::path::Path::new(schema_str).exists() {
        let content = std::fs::read_to_string(schema_str).map_err(|e| {
            anyhow::anyhow!("Failed to read JSON schema file '{}': {}", schema_str, e)
        })?;
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in schema file '{}': {}", schema_str, e))?
    } else {
        serde_json::from_str::<serde_json::Value>(schema_str)
            .map_err(|e| anyhow::anyhow!("Invalid JSON schema: {}", e))?
    };
    json_validation::validate_schema(&schema_json).map_err(|e| anyhow::anyhow!("{}", e))?;
    debug!(
        "JSON schema loaded: {} bytes",
        serde_json::to_string(&schema_json)
            .unwrap_or_default()
            .len()
    );
    Ok(schema_json)
}
