//! High-level builder API for driving agents programmatically.
//!
//! Instead of shelling out to the `agent` CLI binary, Rust programs can
//! use `AgentBuilder` to configure and execute agent sessions directly.
//!
//! # Examples
//!
//! ```no_run
//! use zag_agent::builder::AgentBuilder;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Non-interactive exec — returns structured output
//! let output = AgentBuilder::new()
//!     .provider("claude")
//!     .model("sonnet")
//!     .auto_approve(true)
//!     .exec("write a hello world program")
//!     .await?;
//!
//! println!("{}", output.result.unwrap_or_default());
//!
//! // Interactive session
//! AgentBuilder::new()
//!     .provider("claude")
//!     .run(Some("initial prompt"))
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::agent::Agent;
use crate::attachment::{self, Attachment};
use crate::config::Config;
use crate::factory::AgentFactory;
use crate::json_validation;
use crate::listen::{self, ListenFormat};
use crate::output::AgentOutput;
use crate::process_registration::{self, ProcessRegistration, RegisterOptionsOwned};
use crate::progress::{ProgressHandler, SilentProgress};
use crate::providers::claude::Claude;
use crate::providers::ollama::Ollama;
use crate::sandbox::SandboxConfig;
use crate::session::{SessionEntry, SessionStore};
use crate::session_log::{
    AgentLogEvent, LiveLogContext, LogEventCallback, SessionLogCoordinator, SessionLogMetadata,
    live_adapter_for_provider, logs_dir,
};
use crate::streaming::StreamingSession;
use crate::worktree;
use anyhow::{Result, bail};
use log::{debug, warn};
use std::sync::Arc;
use std::time::Duration;

/// Format a Duration as a human-readable string (e.g., "5m", "1h30m").
fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    let mut parts = Vec::new();
    if h > 0 {
        parts.push(format!("{h}h"));
    }
    if m > 0 {
        parts.push(format!("{m}m"));
    }
    if s > 0 || parts.is_empty() {
        parts.push(format!("{s}s"));
    }
    parts.join("")
}

/// Session discovery metadata — mirrors the `--name`, `--description`, and
/// `--tag` flags on the `run`/`exec`/`spawn` CLI commands. Attached to a
/// builder via [`AgentBuilder::name`], [`AgentBuilder::description`], and
/// [`AgentBuilder::tag`].
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// Private guard returned by `AgentBuilder::start_session_log` — owns
/// the coordinator (when `Auto`) or defers ownership to the caller (when
/// `External`). Dropping the guard implicitly finalises the owned
/// coordinator via its own `Drop` impl.
struct SessionLogGuard {
    /// Set when the builder started its own coordinator (`Auto`). Dropped
    /// at the end of the terminal method, which finalises the log.
    coordinator: Option<SessionLogCoordinator>,
    wrapper_session_id: String,
    log_path: Option<std::path::PathBuf>,
    /// When the caller supplied an `External` coordinator, we keep a
    /// writer clone so that `clear_event_callback` still works on exit.
    external_writer: Option<crate::session_log::SessionLogWriter>,
    /// Holds the externally-owned coordinator until the guard drops so
    /// callers who pass `SessionLogMode::External` don't have to keep
    /// their own handle alive. (They can; this is just a convenience.)
    _owned_external: Option<SessionLogCoordinator>,
}

impl SessionLogGuard {
    fn log_path_string(&self) -> Option<String> {
        self.log_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Flush the coordinator (emit `SessionEnded`, tear down the heartbeat /
    /// live-adapter task). For `External` mode the caller retains ownership
    /// — we merely detach our event callback so it stops firing after the
    /// terminal method returns.
    async fn finish(mut self, success: bool, error: Option<String>) {
        // Run the finalization *before* detaching the callback so that the
        // closing `SessionEnded` event still fires through the user's hook.
        if let Some(coord) = self.coordinator.take() {
            if let Err(e) = coord.finish(success, error).await {
                warn!("Failed to finalize session log: {e}");
            }
        }
        if let Some(w) = self.external_writer.take() {
            let _ = w.clear_event_callback();
        }
    }
}

impl Drop for SessionLogGuard {
    fn drop(&mut self) {
        // Drop-path fallback: if a terminal method panicked or returned
        // early without calling `finish`, still detach callbacks so user
        // code stops receiving events. The owned coordinator's own `Drop`
        // will kill the background task even without an explicit finish.
        if let Some(ref w) = self.external_writer {
            let _ = w.clear_event_callback();
        }
        if let Some(ref c) = self.coordinator {
            let _ = c.writer().clear_event_callback();
        }
    }
}

/// Controls whether the builder manages a [`SessionLogCoordinator`] for the
/// session it launches.
///
/// Default for [`AgentBuilder`] is [`SessionLogMode::Disabled`] so that
/// existing Rust library callers see no side effects. The CLI and any
/// caller that wants live event streaming should select
/// [`SessionLogMode::Auto`].
///
/// [`SessionLogMode::External`] lets an advanced caller (e.g. the CLI's
/// plan/review handlers, `zag-serve`) start and bookkeep its own
/// coordinator — the builder will write through it without double-starting.
#[derive(Default)]
pub enum SessionLogMode {
    /// No session log is started by the builder. No on-disk JSONL and no
    /// live event callbacks.
    #[default]
    Disabled,
    /// The builder starts its own [`SessionLogCoordinator`], tears it down
    /// when the terminal method returns, and populates
    /// [`AgentOutput::log_path`].
    Auto,
    /// The caller provides a pre-started [`SessionLogCoordinator`]; the
    /// builder uses it verbatim and does not stop it at exit.
    External(SessionLogCoordinator),
}

/// Builder for configuring and running agent sessions.
///
/// Use the builder pattern to set options, then call a terminal method
/// (`exec`, `run`, `resume`, `continue_last`) to execute.
pub struct AgentBuilder {
    provider: Option<String>,
    /// Set to true when the caller explicitly pinned a provider via
    /// `.provider()`. When false (default), the fallback tier list is
    /// allowed to downgrade to the next provider on binary/probe failure.
    provider_explicit: bool,
    model: Option<String>,
    system_prompt: Option<String>,
    root: Option<String>,
    auto_approve: bool,
    add_dirs: Vec<String>,
    files: Vec<String>,
    env_vars: Vec<(String, String)>,
    worktree: Option<Option<String>>,
    sandbox: Option<Option<String>>,
    size: Option<String>,
    json_mode: bool,
    json_schema: Option<serde_json::Value>,
    session_id: Option<String>,
    metadata: SessionMetadata,
    output_format: Option<String>,
    input_format: Option<String>,
    replay_user_messages: bool,
    include_partial_messages: bool,
    verbose: bool,
    quiet: bool,
    show_usage: bool,
    max_turns: Option<u32>,
    timeout: Option<std::time::Duration>,
    mcp_config: Option<String>,
    progress: Box<dyn ProgressHandler>,
    session_log_mode: SessionLogMode,
    /// Registered via [`AgentBuilder::on_log_event`] — fired for each
    /// `AgentLogEvent` written to the session log while the terminal method
    /// runs. Requires `session_log_mode != Disabled`.
    log_event_callback: Option<LogEventCallback>,
    /// Set via [`AgentBuilder::stream_events_to_stderr`] — overrides
    /// `log_event_callback` to format and print each event to `stderr`.
    stream_events_format: Option<ListenFormat>,
    /// Whether the built-in stderr streamer should include reasoning events
    /// (set via [`AgentBuilder::stream_show_thinking`]).
    stream_show_thinking: bool,
    /// Registered via [`AgentBuilder::on_spawn`] — invoked once with the
    /// OS pid of the spawned agent subprocess right after spawn, before
    /// the terminal wait. Useful for registering the child pid with an
    /// external process store.
    on_spawn_hook: Option<crate::agent::OnSpawnHook>,
    /// Set via [`AgentBuilder::register_process`] — when present, the
    /// terminal method registers a `ProcessEntry` in zag's `ProcessStore`,
    /// injects `ZAG_PROCESS_ID` / `ZAG_SESSION_ID` etc. into the agent
    /// subprocess's env, retargets the entry's pid to the agent child via
    /// an internal `on_spawn` hook, and finalises the entry's status when
    /// the terminal method returns.
    register_process_opts: Option<RegisterOptionsOwned>,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            provider: None,
            provider_explicit: false,
            model: None,
            system_prompt: None,
            root: None,
            auto_approve: false,
            add_dirs: Vec::new(),
            files: Vec::new(),
            env_vars: Vec::new(),
            worktree: None,
            sandbox: None,
            size: None,
            json_mode: false,
            json_schema: None,
            session_id: None,
            metadata: SessionMetadata::default(),
            output_format: None,
            input_format: None,
            replay_user_messages: false,
            include_partial_messages: false,
            verbose: false,
            quiet: false,
            show_usage: false,
            max_turns: None,
            timeout: None,
            mcp_config: None,
            progress: Box::new(SilentProgress),
            session_log_mode: SessionLogMode::Disabled,
            log_event_callback: None,
            stream_events_format: None,
            stream_show_thinking: false,
            on_spawn_hook: None,
            register_process_opts: None,
        }
    }

    /// Set the provider (e.g., "claude", "codex", "gemini", "copilot", "ollama").
    ///
    /// Calling this method pins the provider — it will NOT be downgraded to
    /// another provider in the tier list if its binary is missing or the
    /// startup probe fails. Omit this call (or set `provider` via the config
    /// file) to allow automatic downgrading.
    pub fn provider(mut self, provider: &str) -> Self {
        self.provider = Some(provider.to_string());
        self.provider_explicit = true;
        self
    }

    /// Set the model (e.g., "sonnet", "opus", "small", "large").
    pub fn model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    /// Set a system prompt to configure agent behavior.
    pub fn system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Set the root directory for the agent to operate in.
    pub fn root(mut self, root: &str) -> Self {
        self.root = Some(root.to_string());
        self
    }

    /// Enable auto-approve mode (skip permission prompts).
    pub fn auto_approve(mut self, approve: bool) -> Self {
        self.auto_approve = approve;
        self
    }

    /// Add an additional directory for the agent to include.
    pub fn add_dir(mut self, dir: &str) -> Self {
        self.add_dirs.push(dir.to_string());
        self
    }

    /// Attach a file to the prompt (text files ≤50 KB inlined, others referenced).
    pub fn file(mut self, path: &str) -> Self {
        self.files.push(path.to_string());
        self
    }

    /// Add an environment variable for the agent subprocess.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.push((key.to_string(), value.to_string()));
        self
    }

    /// Enable worktree mode with an optional name.
    pub fn worktree(mut self, name: Option<&str>) -> Self {
        self.worktree = Some(name.map(String::from));
        self
    }

    /// Enable sandbox mode with an optional name.
    pub fn sandbox(mut self, name: Option<&str>) -> Self {
        self.sandbox = Some(name.map(String::from));
        self
    }

    /// Set the Ollama parameter size (e.g., "2b", "9b", "35b").
    pub fn size(mut self, size: &str) -> Self {
        self.size = Some(size.to_string());
        self
    }

    /// Request JSON output from the agent.
    pub fn json(mut self) -> Self {
        self.json_mode = true;
        self
    }

    /// Set a JSON schema for structured output validation.
    /// Implies `json()`.
    pub fn json_schema(mut self, schema: serde_json::Value) -> Self {
        self.json_schema = Some(schema);
        self.json_mode = true;
        self
    }

    /// Set a specific session ID (UUID).
    pub fn session_id(mut self, id: &str) -> Self {
        self.session_id = Some(id.to_string());
        self
    }

    /// Set a human-readable session name (mirrors the CLI's `--name`).
    ///
    /// Names are used by `zag input --name <n>`, `zag session list --name
    /// <n>`, and for session discovery across the store. When the session
    /// has a generated wrapper ID, the builder will persist this name to
    /// the session store so CLI tools can find it later.
    pub fn name(mut self, name: &str) -> Self {
        self.metadata.name = Some(name.to_string());
        self
    }

    /// Set a short description for the session (mirrors the CLI's
    /// `--description`).
    pub fn description(mut self, description: &str) -> Self {
        self.metadata.description = Some(description.to_string());
        self
    }

    /// Add a discovery tag for the session (mirrors the CLI's `--tag`,
    /// repeatable).
    pub fn tag(mut self, tag: &str) -> Self {
        self.metadata.tags.push(tag.to_string());
        self
    }

    /// Replace the full session metadata in one call.
    pub fn metadata(mut self, metadata: SessionMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the output format (e.g., "text", "json", "json-pretty", "stream-json").
    pub fn output_format(mut self, format: &str) -> Self {
        self.output_format = Some(format.to_string());
        self
    }

    /// Set the input format (Claude only, e.g., "text", "stream-json").
    ///
    /// No-op for Codex, Gemini, Copilot, and Ollama. See `docs/providers.md`
    /// for the full per-provider support matrix.
    pub fn input_format(mut self, format: &str) -> Self {
        self.input_format = Some(format.to_string());
        self
    }

    /// Re-emit user messages from stdin on stdout (Claude only).
    ///
    /// Only works with `--input-format stream-json` and `--output-format stream-json`.
    /// [`exec_streaming`](Self::exec_streaming) auto-enables this flag, so most
    /// callers never need to set it manually. No-op for non-Claude providers.
    pub fn replay_user_messages(mut self, replay: bool) -> Self {
        self.replay_user_messages = replay;
        self
    }

    /// Include partial message chunks in streaming output (Claude only).
    ///
    /// Only works with `--output-format stream-json`. Defaults to `false`.
    ///
    /// When `false` (the default), streaming surfaces one `assistant_message`
    /// event per complete assistant turn. When `true`, the agent instead emits
    /// a stream of token-level partial `assistant_message` chunks as the model
    /// generates them — use this for responsive, token-by-token UIs over
    /// [`exec_streaming`](Self::exec_streaming). No-op for non-Claude providers.
    pub fn include_partial_messages(mut self, include: bool) -> Self {
        self.include_partial_messages = include;
        self
    }

    /// Enable verbose output.
    pub fn verbose(mut self, v: bool) -> Self {
        self.verbose = v;
        self
    }

    /// Enable quiet mode (suppress all non-essential output).
    pub fn quiet(mut self, q: bool) -> Self {
        self.quiet = q;
        self
    }

    /// Show token usage statistics.
    pub fn show_usage(mut self, show: bool) -> Self {
        self.show_usage = show;
        self
    }

    /// Set the maximum number of agentic turns.
    pub fn max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Set a timeout for exec. If the agent doesn't complete within this
    /// duration, it will be killed and an error returned.
    pub fn timeout(mut self, duration: std::time::Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set MCP server config for this invocation (Claude only).
    ///
    /// Accepts either a JSON string (`{"mcpServers": {...}}`) or a path to a JSON file.
    /// No-op for Codex, Gemini, Copilot, and Ollama — those providers manage
    /// MCP configuration through their own CLIs or do not support it. See
    /// `docs/providers.md` for the full per-provider support matrix.
    pub fn mcp_config(mut self, config: &str) -> Self {
        self.mcp_config = Some(config.to_string());
        self
    }

    /// Set a custom progress handler for status reporting.
    pub fn on_progress(mut self, handler: Box<dyn ProgressHandler>) -> Self {
        self.progress = handler;
        self
    }

    /// Select how the builder manages the session log. See [`SessionLogMode`].
    pub fn session_log(mut self, mode: SessionLogMode) -> Self {
        self.session_log_mode = mode;
        self
    }

    /// Shortcut for `.session_log(SessionLogMode::Auto)` when `true`, or
    /// `.session_log(SessionLogMode::Disabled)` when `false`.
    pub fn enable_session_log(mut self, enable: bool) -> Self {
        self.session_log_mode = if enable {
            SessionLogMode::Auto
        } else {
            SessionLogMode::Disabled
        };
        self
    }

    /// Register a callback fired for each `AgentLogEvent` written to the
    /// session log during the terminal method. Implicitly switches
    /// `session_log_mode` to [`SessionLogMode::Auto`] if it is currently
    /// [`SessionLogMode::Disabled`].
    pub fn on_log_event<F>(mut self, f: F) -> Self
    where
        F: Fn(&AgentLogEvent) + Send + Sync + 'static,
    {
        self.log_event_callback = Some(Arc::new(f));
        if matches!(self.session_log_mode, SessionLogMode::Disabled) {
            self.session_log_mode = SessionLogMode::Auto;
        }
        self
    }

    /// Convenience: tail the session log to stderr during the terminal
    /// method, using the same formatters as the `zag listen` command.
    ///
    /// This is the drop-in replacement for the live stderr tail that a
    /// previous shell-out-to-`zag` wrapper produced. Implicitly enables
    /// session logging.
    pub fn stream_events_to_stderr(mut self, format: ListenFormat) -> Self {
        self.stream_events_format = Some(format);
        if matches!(self.session_log_mode, SessionLogMode::Disabled) {
            self.session_log_mode = SessionLogMode::Auto;
        }
        self
    }

    /// Include `Reasoning` events in the stderr stream when
    /// [`stream_events_to_stderr`] is active. Off by default.
    pub fn stream_show_thinking(mut self, show: bool) -> Self {
        self.stream_show_thinking = show;
        self
    }

    /// Register a callback invoked once with the OS pid of the spawned
    /// agent subprocess, right after spawn and before the terminal
    /// wait. Useful for registering the child pid with an external
    /// process store so `zag ps kill self` (or equivalent) can SIGTERM
    /// the agent child rather than the parent zag process.
    ///
    /// The callback fires once per spawn — on retries or resumes it
    /// fires again for each new child. See [`crate::agent::OnSpawnHook`]
    /// for the full semantics.
    pub fn on_spawn<F>(mut self, f: F) -> Self
    where
        F: Fn(u32) + Send + Sync + 'static,
    {
        self.on_spawn_hook = Some(Arc::new(f));
        self
    }

    /// Register the about-to-spawn agent in zag's `ProcessStore` and inject
    /// `ZAG_PROCESS_ID` / `ZAG_SESSION_ID` / `ZAG_PROVIDER` / `ZAG_MODEL`
    /// into its env so commands like `zag ps kill self` and `zig self
    /// terminate` can resolve the running agent from inside its own
    /// subshell.
    ///
    /// The terminal method takes care of:
    ///
    /// 1. Calling [`crate::process_registration::register`] before spawn.
    /// 2. Appending the resulting env vars onto the builder.
    /// 3. Composing the registration's `on_spawn` hook with any caller-set
    ///    [`AgentBuilder::on_spawn`] so both fire.
    /// 4. Calling [`ProcessRegistration::update_status`] with `"exited"` /
    ///    `"killed"` once the agent finishes.
    ///
    /// Without this call, the builder leaves the registry untouched (the
    /// pre-existing default behavior). Callers that already manage their
    /// own `ProcessEntry` should not opt in.
    pub fn register_process(mut self, opts: RegisterOptionsOwned) -> Self {
        self.register_process_opts = Some(opts);
        self
    }
}

/// Apply a [`ProcessRegistration`] to the builder's `env_vars` and
/// `on_spawn_hook` fields so the agent subprocess inherits the env vars and
/// the registry entry's pid is retargeted on spawn. Composes with any
/// caller-set `on_spawn` hook — both fire.
fn apply_registration(builder: &mut AgentBuilder, reg: &ProcessRegistration) {
    for (k, v) in reg.env_vars() {
        builder.env_vars.push((k.clone(), v.clone()));
    }
    let reg_hook = reg.on_spawn_hook();
    let prev_hook = builder.on_spawn_hook.take();
    builder.on_spawn_hook = Some(Arc::new(move |pid: u32| {
        reg_hook(pid);
        if let Some(ref h) = prev_hook {
            h(pid);
        }
    }));
}

/// Map a terminal-method `Result<T>` to the `(status, exit_code)` pair
/// stored on the `ProcessEntry`. Mirrors what `zag-cli/src/commands/
/// agent_action.rs` records: `"killed"` + the agent's reported exit code on
/// failure, `"exited"` + 0 on success.
fn status_for_result<T>(result: &Result<T>) -> (&'static str, Option<i32>) {
    match result {
        Ok(_) => ("exited", Some(0)),
        Err(err) => {
            let exit_code = err
                .downcast_ref::<crate::process::ProcessError>()
                .and_then(|pe| pe.exit_code)
                .unwrap_or(1);
            ("killed", Some(exit_code))
        }
    }
}

impl AgentBuilder {
    /// Persist a `SessionEntry` to the session store so `zag session list`
    /// and `zag input --name <n>` can discover this builder-spawned session.
    ///
    /// No-op when no metadata is set — callers who don't name their sessions
    /// still get the old behavior of not leaving a trail in the session store.
    ///
    /// Returns the session ID that was persisted (either the caller-provided
    /// one or a freshly generated UUID), so downstream logging can reference
    /// the same ID.
    fn persist_session_metadata_with_id(
        &self,
        provider: &str,
        model: &str,
        effective_root: Option<&str>,
        explicit_session_id: Option<&str>,
    ) -> Option<String> {
        let has_metadata = self.metadata.name.is_some()
            || self.metadata.description.is_some()
            || !self.metadata.tags.is_empty();
        if !has_metadata {
            return None;
        }

        let session_id = explicit_session_id
            .map(String::from)
            .or_else(|| self.session_id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let workspace_path = effective_root
            .map(String::from)
            .or_else(|| self.root.clone())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default()
            });

        let entry = SessionEntry {
            session_id: session_id.clone(),
            provider: provider.to_string(),
            model: model.to_string(),
            worktree_path: workspace_path,
            worktree_name: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            provider_session_id: None,
            sandbox_name: None,
            is_worktree: self.worktree.is_some(),
            discovered: false,
            discovery_source: None,
            log_path: None,
            log_completeness: "partial".to_string(),
            name: self.metadata.name.clone(),
            description: self.metadata.description.clone(),
            tags: self.metadata.tags.clone(),
            dependencies: Vec::new(),
            retried_from: None,
            interactive: false,
        };

        let mut store = SessionStore::load(self.root.as_deref()).unwrap_or_default();
        store.add(entry);
        if let Err(e) = store.save(self.root.as_deref()) {
            warn!("Failed to persist session metadata: {e}");
        }

        Some(session_id)
    }

    /// Resolve file attachments and prepend them to a prompt.
    fn prepend_files(&self, prompt: &str) -> Result<String> {
        if self.files.is_empty() {
            return Ok(prompt.to_string());
        }
        let attachments: Vec<Attachment> = self
            .files
            .iter()
            .map(|f| Attachment::from_path(std::path::Path::new(f)))
            .collect::<Result<Vec<_>>>()?;
        let prefix = attachment::format_attachments_prefix(&attachments);
        Ok(format!("{prefix}{prompt}"))
    }

    /// Resolve the effective provider name.
    fn resolve_provider(&self) -> Result<String> {
        if let Some(ref p) = self.provider {
            let p = p.to_lowercase();
            if !Config::VALID_PROVIDERS.contains(&p.as_str()) {
                bail!(
                    "Invalid provider '{}'. Available: {}",
                    p,
                    Config::VALID_PROVIDERS.join(", ")
                );
            }
            return Ok(p);
        }
        let config = Config::load(self.root.as_deref()).unwrap_or_default();
        if let Some(p) = config.provider() {
            return Ok(p.to_string());
        }
        Ok("claude".to_string())
    }

    /// Create and configure the agent.
    ///
    /// Returns the constructed agent along with the provider name that
    /// actually succeeded. When `provider_explicit` is false, the factory
    /// may downgrade to another provider in the tier list, so the returned
    /// provider can differ from the one passed in.
    async fn create_agent(&self, provider: &str) -> Result<(Box<dyn Agent + Send + Sync>, String)> {
        // Apply system_prompt config fallback
        let base_system_prompt = self.system_prompt.clone().or_else(|| {
            Config::load(self.root.as_deref())
                .unwrap_or_default()
                .system_prompt()
                .map(String::from)
        });

        // Augment system prompt with JSON instructions for non-Claude agents
        let system_prompt = if self.json_mode && provider != "claude" {
            let mut prompt = base_system_prompt.unwrap_or_default();
            if let Some(ref schema) = self.json_schema {
                let schema_str = serde_json::to_string_pretty(schema).unwrap_or_default();
                prompt.push_str(&format!(
                    "\n\nYou MUST respond with valid JSON only. No markdown fences, no explanations. \
                     Your response must conform to this JSON schema:\n{schema_str}"
                ));
            } else {
                prompt.push_str(
                    "\n\nYou MUST respond with valid JSON only. No markdown fences, no explanations.",
                );
            }
            Some(prompt)
        } else {
            base_system_prompt
        };

        self.progress
            .on_spinner_start(&format!("Initializing {provider} agent"));

        let progress = &*self.progress;
        let mut on_downgrade = |from: &str, to: &str, reason: &str| {
            progress.on_warning(&format!("Downgrading provider: {from} → {to} ({reason})"));
        };
        let (mut agent, effective_provider) = AgentFactory::create_with_fallback(
            provider,
            self.provider_explicit,
            system_prompt,
            self.model.clone(),
            self.root.clone(),
            self.auto_approve,
            self.add_dirs.clone(),
            &mut on_downgrade,
        )
        .await?;
        let provider = effective_provider.as_str();

        // Apply max_turns: explicit > config > none
        let effective_max_turns = self.max_turns.or_else(|| {
            Config::load(self.root.as_deref())
                .unwrap_or_default()
                .max_turns()
        });
        if let Some(turns) = effective_max_turns {
            agent.set_max_turns(turns);
        }

        // Set output format
        let mut output_format = self.output_format.clone();
        if self.json_mode && output_format.is_none() {
            output_format = Some("json".to_string());
            if provider != "claude" {
                agent.set_capture_output(true);
            }
        }
        agent.set_output_format(output_format);

        // Configure Claude-specific options
        if provider == "claude"
            && let Some(claude_agent) = agent.as_any_mut().downcast_mut::<Claude>()
        {
            claude_agent.set_verbose(self.verbose);
            if let Some(ref session_id) = self.session_id {
                claude_agent.set_session_id(session_id.clone());
            }
            if let Some(ref input_fmt) = self.input_format {
                claude_agent.set_input_format(Some(input_fmt.clone()));
            }
            if self.replay_user_messages {
                claude_agent.set_replay_user_messages(true);
            }
            if self.include_partial_messages {
                claude_agent.set_include_partial_messages(true);
            }
            if self.json_mode
                && let Some(ref schema) = self.json_schema
            {
                let schema_str = serde_json::to_string(schema).unwrap_or_default();
                claude_agent.set_json_schema(Some(schema_str));
            }
            if self.mcp_config.is_some() {
                claude_agent.set_mcp_config(self.mcp_config.clone());
            }
        }

        // Configure Ollama-specific options
        if provider == "ollama"
            && let Some(ollama_agent) = agent.as_any_mut().downcast_mut::<Ollama>()
        {
            let config = Config::load(self.root.as_deref()).unwrap_or_default();
            if let Some(ref size) = self.size {
                let resolved = config.ollama_size_for(size);
                ollama_agent.set_size(resolved.to_string());
            }
        }

        // Configure sandbox
        if let Some(ref sandbox_opt) = self.sandbox {
            let sandbox_name = sandbox_opt
                .as_deref()
                .map(String::from)
                .unwrap_or_else(crate::sandbox::generate_name);
            let template = crate::sandbox::template_for_provider(provider);
            let workspace = self.root.clone().unwrap_or_else(|| ".".to_string());
            agent.set_sandbox(SandboxConfig {
                name: sandbox_name,
                template: template.to_string(),
                workspace,
            });
        }

        if !self.env_vars.is_empty() {
            agent.set_env_vars(self.env_vars.clone());
        }

        if let Some(ref hook) = self.on_spawn_hook {
            agent.set_on_spawn_hook(hook.clone());
        }

        self.progress.on_spinner_finish();
        self.progress.on_success(&format!(
            "{} initialized with model {}",
            provider,
            agent.get_model()
        ));

        Ok((agent, effective_provider))
    }

    /// Start (or adopt) a [`SessionLogCoordinator`] for the session about
    /// to run, honouring the builder's [`SessionLogMode`] and wiring up any
    /// registered `on_log_event` / `stream_events_to_stderr` callback.
    ///
    /// Returns a guard that owns the coordinator (where applicable) and the
    /// resolved `wrapper_session_id` + log path, or `None` when logging is
    /// disabled.
    fn start_session_log(
        &mut self,
        command: &str,
        resumed: bool,
        provider: &str,
        model: &str,
    ) -> Option<SessionLogGuard> {
        let mode = std::mem::replace(&mut self.session_log_mode, SessionLogMode::Disabled);
        match mode {
            SessionLogMode::Disabled => None,
            SessionLogMode::External(c) => {
                let wrapper_session_id = c
                    .writer()
                    .log_path()
                    .ok()
                    .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_default();
                let log_path = c.writer().log_path().ok();
                self.apply_event_callback(c.writer());
                Some(SessionLogGuard {
                    coordinator: None, // externally owned
                    wrapper_session_id,
                    log_path,
                    external_writer: Some(c.writer().clone()),
                    _owned_external: Some(c),
                })
            }
            SessionLogMode::Auto => {
                let wrapper_session_id = self
                    .session_id
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let metadata = SessionLogMetadata {
                    provider: provider.to_string(),
                    wrapper_session_id: wrapper_session_id.clone(),
                    provider_session_id: None,
                    workspace_path: self.root.clone().or_else(|| {
                        std::env::current_dir()
                            .ok()
                            .map(|p| p.to_string_lossy().to_string())
                    }),
                    command: command.to_string(),
                    model: Some(model.to_string()),
                    resumed,
                    backfilled: false,
                };
                let live_ctx = LiveLogContext {
                    root: self.root.clone(),
                    provider_session_id: metadata.provider_session_id.clone(),
                    workspace_path: metadata.workspace_path.clone(),
                    started_at: chrono::Utc::now(),
                    is_worktree: self.worktree.is_some(),
                };
                let adapter = live_adapter_for_provider(provider, live_ctx, true);
                let callback = self.build_event_callback();
                match SessionLogCoordinator::start_with_callback(
                    &logs_dir(self.root.as_deref()),
                    metadata,
                    adapter,
                    callback,
                ) {
                    Ok(c) => {
                        let _ = c.writer().set_global_index_dir(Config::global_base_dir());
                        let log_path = c.writer().log_path().ok();
                        Some(SessionLogGuard {
                            coordinator: Some(c),
                            wrapper_session_id,
                            log_path,
                            external_writer: None,
                            _owned_external: None,
                        })
                    }
                    Err(e) => {
                        warn!("Failed to start session log coordinator: {e}");
                        None
                    }
                }
            }
        }
    }

    /// Build a combined event callback from any registered `on_log_event`
    /// and `stream_events_to_stderr` setters. Returns `None` when neither
    /// is set so the writer doesn't pay any per-event cost.
    fn build_event_callback(&self) -> Option<LogEventCallback> {
        let user_cb = self.log_event_callback.clone();
        let stream_fmt = self.stream_events_format;
        let show_thinking = self.stream_show_thinking;

        if user_cb.is_none() && stream_fmt.is_none() {
            return None;
        }

        Some(Arc::new(move |event: &AgentLogEvent| {
            if let Some(ref user) = user_cb {
                user(event);
            }
            if let Some(fmt) = stream_fmt
                && let Some(text) = listen::format_event(event, fmt, show_thinking)
            {
                eprintln!("{text}");
            }
        }))
    }

    /// Register the builder's callback on an externally-owned writer (used
    /// by [`SessionLogMode::External`] — the coordinator is already running
    /// so we can't register the callback before `SessionStarted`, but any
    /// post-adoption event will still fire).
    fn apply_event_callback(&self, writer: &crate::session_log::SessionLogWriter) {
        if let Some(cb) = self.build_event_callback() {
            if let Err(e) = writer.set_event_callback(cb) {
                warn!("Failed to register session log event callback: {e}");
            }
        }
    }

    /// Run the agent non-interactively and return structured output.
    ///
    /// This is the primary entry point for programmatic use.
    pub async fn exec(mut self, prompt: &str) -> Result<AgentOutput> {
        let registration = self
            .register_process_opts
            .as_ref()
            .map(|opts| process_registration::register(opts.as_borrowed()));
        if let Some(ref reg) = registration {
            apply_registration(&mut self, reg);
        }
        let result = self.exec_inner(prompt).await;
        if let Some(reg) = registration {
            let (status, code) = status_for_result(&result);
            reg.update_status(status, code);
        }
        result
    }

    async fn exec_inner(self, prompt: &str) -> Result<AgentOutput> {
        let provider = self.resolve_provider()?;
        debug!("exec: provider={provider}");

        // Set up worktree if requested
        let effective_root = if let Some(ref wt_opt) = self.worktree {
            let wt_name = wt_opt
                .as_deref()
                .map(String::from)
                .unwrap_or_else(worktree::generate_name);
            let repo_root = worktree::git_repo_root(self.root.as_deref())?;
            let wt_path = worktree::create_worktree(&repo_root, &wt_name)?;
            self.progress
                .on_success(&format!("Worktree created at {}", wt_path.display()));
            Some(wt_path.to_string_lossy().to_string())
        } else {
            self.root.clone()
        };

        let mut builder = self;
        if effective_root.is_some() {
            builder.root = effective_root;
        }

        let (agent, provider) = builder.create_agent(&provider).await?;

        // Start (or adopt) the session log coordinator. Held for the whole
        // terminal method; dropped after cleanup so the log file is
        // finalised exactly once.
        let log_guard = builder.start_session_log("exec", false, &provider, agent.get_model());

        // Persist the session entry so discovery (session list --name, input
        // --name) works for builder-spawned sessions. No-op when no metadata
        // is set. When session logging is active, share its wrapper_session_id
        // so the store entry and the JSONL log agree.
        let _ = builder.persist_session_metadata_with_id(
            &provider,
            agent.get_model(),
            builder.root.as_deref(),
            log_guard.as_ref().map(|g| g.wrapper_session_id.as_str()),
        );

        // Prepend file attachments
        let prompt_with_files = builder.prepend_files(prompt)?;

        // Handle JSON mode with prompt wrapping for non-Claude agents
        let effective_prompt = if builder.json_mode && provider != "claude" {
            format!(
                "IMPORTANT: You MUST respond with valid JSON only. No markdown, no explanation.\n\n{prompt_with_files}"
            )
        } else {
            prompt_with_files
        };

        let result = if let Some(timeout_dur) = builder.timeout {
            match tokio::time::timeout(timeout_dur, agent.run(Some(&effective_prompt))).await {
                Ok(r) => r?,
                Err(_) => {
                    agent.cleanup().await.ok();
                    bail!("Agent timed out after {}", format_duration(timeout_dur));
                }
            }
        } else {
            agent.run(Some(&effective_prompt)).await?
        };

        // Clean up
        agent.cleanup().await?;

        let log_path_string = log_guard.as_ref().and_then(|g| g.log_path_string());

        if let Some(mut output) = result {
            // Validate JSON output if schema is provided
            if let Some(ref schema) = builder.json_schema {
                if !builder.json_mode {
                    warn!(
                        "json_schema is set but json_mode is false — \
                         schema will not be sent to the agent, only used for output validation"
                    );
                }
                if let Some(ref result_text) = output.result {
                    debug!(
                        "exec: validating result ({} bytes): {:.300}",
                        result_text.len(),
                        result_text
                    );
                    if let Err(errors) = json_validation::validate_json_schema(result_text, schema)
                    {
                        let preview = if result_text.len() > 500 {
                            &result_text[..500]
                        } else {
                            result_text.as_str()
                        };
                        bail!(
                            "JSON schema validation failed: {}\nRaw agent output ({} bytes):\n{}",
                            errors.join("; "),
                            result_text.len(),
                            preview
                        );
                    }
                }
            }
            output.log_path = log_path_string;
            let success = !output.is_error;
            let err_msg = output.error_message.clone();
            if let Some(g) = log_guard {
                g.finish(success, err_msg).await;
            }
            Ok(output)
        } else {
            // Agent returned no structured output — create a minimal one
            let mut output = AgentOutput::from_text(&provider, "");
            output.log_path = log_path_string;
            if let Some(g) = log_guard {
                g.finish(true, None).await;
            }
            Ok(output)
        }
    }

    /// Run the agent with streaming input and output (Claude only).
    ///
    /// Returns a [`StreamingSession`] that allows sending NDJSON messages to
    /// the agent's stdin and reading events from stdout. Automatically
    /// configures `--input-format stream-json`, `--output-format stream-json`,
    /// and `--replay-user-messages`.
    ///
    /// # Default emission granularity
    ///
    /// By default `assistant_message` events are emitted **once per complete
    /// assistant turn** — you get one event when the model finishes speaking,
    /// not a stream of token chunks. For responsive, token-level UIs call
    /// [`include_partial_messages(true)`](Self::include_partial_messages)
    /// on the builder before `exec_streaming`; the session will then emit
    /// partial `assistant_message` chunks as the model generates them.
    ///
    /// The default is kept `false` so existing callers that render whole-turn
    /// bubbles are not broken. See `docs/providers.md` for the full
    /// per-provider flag support matrix.
    ///
    /// # Event lifecycle
    ///
    /// The session emits a unified
    /// [`Event::Result`](crate::output::Event::Result) at the **end of every
    /// agent turn** — not only at final session end. Use that event as the
    /// authoritative turn-boundary signal. After a `Result`, the session
    /// remains open and accepts another
    /// [`send_user_message`](StreamingSession::send_user_message) for the next
    /// turn. Call
    /// [`close_input`](StreamingSession::close_input) followed by
    /// [`wait`](StreamingSession::wait) to terminate the session cleanly.
    ///
    /// Do not depend on replayed `user_message` events to detect turn
    /// boundaries; those only appear while `--replay-user-messages` is set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zag_agent::builder::AgentBuilder;
    /// use zag_agent::output::Event;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let mut session = AgentBuilder::new()
    ///     .provider("claude")
    ///     .exec_streaming("initial prompt")
    ///     .await?;
    ///
    /// // Drain the first turn until Result.
    /// while let Some(event) = session.next_event().await? {
    ///     println!("{:?}", event);
    ///     if matches!(event, Event::Result { .. }) {
    ///         break;
    ///     }
    /// }
    ///
    /// // Follow-up turn.
    /// session.send_user_message("do something else").await?;
    /// while let Some(event) = session.next_event().await? {
    ///     if matches!(event, Event::Result { .. }) {
    ///         break;
    ///     }
    /// }
    ///
    /// session.close_input();
    /// session.wait().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exec_streaming(self, prompt: &str) -> Result<StreamingSession> {
        let provider = self.resolve_provider()?;
        debug!("exec_streaming: provider={provider}");

        if provider != "claude" {
            bail!("Streaming input is only supported by the Claude provider");
        }

        // Prepend file attachments
        let prompt_with_files = self.prepend_files(prompt)?;

        // Streaming only works on Claude — do not allow the fallback loop
        // to downgrade to a provider that can't stream.
        let mut builder = self;
        builder.provider_explicit = true;
        let (agent, _provider) = builder.create_agent(&provider).await?;

        // Downcast to Claude to call execute_streaming
        let claude_agent = agent
            .as_any_ref()
            .downcast_ref::<Claude>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast agent to Claude"))?;

        claude_agent.execute_streaming(Some(&prompt_with_files))
    }

    /// Start an interactive agent session.
    ///
    /// This takes over stdin/stdout for the duration of the session.
    pub async fn run(mut self, prompt: Option<&str>) -> Result<()> {
        let registration = self
            .register_process_opts
            .as_ref()
            .map(|opts| process_registration::register(opts.as_borrowed()));
        if let Some(ref reg) = registration {
            apply_registration(&mut self, reg);
        }
        let result = self.run_inner(prompt).await;
        if let Some(reg) = registration {
            let (status, code) = status_for_result(&result);
            reg.update_status(status, code);
        }
        result
    }

    async fn run_inner(self, prompt: Option<&str>) -> Result<()> {
        let provider = self.resolve_provider()?;
        debug!("run: provider={provider}");

        // Prepend file attachments
        let prompt_with_files = match prompt {
            Some(p) => Some(self.prepend_files(p)?),
            None if !self.files.is_empty() => {
                let attachments: Vec<Attachment> = self
                    .files
                    .iter()
                    .map(|f| Attachment::from_path(std::path::Path::new(f)))
                    .collect::<Result<Vec<_>>>()?;
                Some(attachment::format_attachments_prefix(&attachments))
            }
            None => None,
        };

        let mut builder = self;
        let (agent, effective_provider) = builder.create_agent(&provider).await?;
        let log_guard =
            builder.start_session_log("run", false, &effective_provider, agent.get_model());
        let _ = builder.persist_session_metadata_with_id(
            &effective_provider,
            agent.get_model(),
            builder.root.as_deref(),
            log_guard.as_ref().map(|g| g.wrapper_session_id.as_str()),
        );
        agent.run_interactive(prompt_with_files.as_deref()).await?;
        agent.cleanup().await?;
        if let Some(g) = log_guard {
            g.finish(true, None).await;
        }
        Ok(())
    }

    /// Resume a previous session by ID.
    pub async fn resume(mut self, session_id: &str) -> Result<()> {
        let registration = self
            .register_process_opts
            .as_ref()
            .map(|opts| process_registration::register(opts.as_borrowed()));
        if let Some(ref reg) = registration {
            apply_registration(&mut self, reg);
        }
        let result = self.resume_inner(session_id).await;
        if let Some(reg) = registration {
            let (status, code) = status_for_result(&result);
            reg.update_status(status, code);
        }
        result
    }

    async fn resume_inner(self, session_id: &str) -> Result<()> {
        let provider = self.resolve_provider()?;
        debug!("resume: provider={provider}, session={session_id}");

        // Resuming must stick with the recorded provider — no downgrade.
        let mut builder = self;
        builder.provider_explicit = true;
        let (agent, effective_provider) = builder.create_agent(&provider).await?;
        let log_guard =
            builder.start_session_log("resume", true, &effective_provider, agent.get_model());
        agent.run_resume(Some(session_id), false).await?;
        agent.cleanup().await?;
        if let Some(g) = log_guard {
            g.finish(true, None).await;
        }
        Ok(())
    }

    /// Resume a previous session and inject a new user message as the next
    /// turn. Captures the agent's response (analogous to [`exec`](Self::exec)).
    ///
    /// Unlike [`resume`](Self::resume), this method is non-interactive — it
    /// does not attach to stdio and instead returns the structured
    /// [`AgentOutput`] produced by the agent for the injected prompt.
    ///
    /// Provider support mirrors the underlying
    /// [`Agent::run_resume_with_prompt`](crate::agent::Agent::run_resume_with_prompt)
    /// trait method: Claude, Codex, and the mock provider implement it; the
    /// trait's default impl errors out for providers that don't, so callers
    /// see a clear "unsupported" message rather than silent misbehavior.
    pub async fn resume_with_prompt(
        mut self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        let registration = self
            .register_process_opts
            .as_ref()
            .map(|opts| process_registration::register(opts.as_borrowed()));
        if let Some(ref reg) = registration {
            apply_registration(&mut self, reg);
        }
        let result = self.resume_with_prompt_inner(session_id, prompt).await;
        if let Some(reg) = registration {
            let (status, code) = status_for_result(&result);
            reg.update_status(status, code);
        }
        result
    }

    async fn resume_with_prompt_inner(
        self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        let provider = self.resolve_provider()?;
        debug!("resume_with_prompt: provider={provider}, session={session_id}");

        // Resuming must stick with the recorded provider — no downgrade.
        let mut builder = self;
        builder.provider_explicit = true;
        let (agent, effective_provider) = builder.create_agent(&provider).await?;
        let log_guard =
            builder.start_session_log("resume", true, &effective_provider, agent.get_model());
        let output = agent.run_resume_with_prompt(session_id, prompt).await?;
        agent.cleanup().await?;
        if let Some(g) = log_guard {
            g.finish(true, None).await;
        }
        Ok(output)
    }

    /// Resume the most recent session.
    pub async fn continue_last(mut self) -> Result<()> {
        let registration = self
            .register_process_opts
            .as_ref()
            .map(|opts| process_registration::register(opts.as_borrowed()));
        if let Some(ref reg) = registration {
            apply_registration(&mut self, reg);
        }
        let result = self.continue_last_inner().await;
        if let Some(reg) = registration {
            let (status, code) = status_for_result(&result);
            reg.update_status(status, code);
        }
        result
    }

    async fn continue_last_inner(self) -> Result<()> {
        let provider = self.resolve_provider()?;
        debug!("continue_last: provider={provider}");

        // Resuming must stick with the recorded provider — no downgrade.
        let mut builder = self;
        builder.provider_explicit = true;
        let (agent, effective_provider) = builder.create_agent(&provider).await?;
        let log_guard =
            builder.start_session_log("resume", true, &effective_provider, agent.get_model());
        agent.run_resume(None, true).await?;
        agent.cleanup().await?;
        if let Some(g) = log_guard {
            g.finish(true, None).await;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
