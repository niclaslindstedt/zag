//! High-level builder API for driving agents programmatically.
//!
//! Instead of shelling out to the `agent` CLI binary, Rust programs can
//! use `AgentBuilder` to configure and execute agent sessions directly.
//!
//! # Examples
//!
//! ```no_run
//! use zag::builder::AgentBuilder;
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
use crate::config::Config;
use crate::factory::AgentFactory;
use crate::json_validation;
use crate::output::AgentOutput;
use crate::progress::{ProgressHandler, SilentProgress};
use crate::providers::claude::Claude;
use crate::providers::ollama::Ollama;
use crate::sandbox::SandboxConfig;
use crate::streaming::StreamingSession;
use crate::worktree;
use anyhow::{Result, bail};
use log::{debug, warn};

/// Builder for configuring and running agent sessions.
///
/// Use the builder pattern to set options, then call a terminal method
/// (`exec`, `run`, `resume`, `continue_last`) to execute.
pub struct AgentBuilder {
    provider: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
    root: Option<String>,
    auto_approve: bool,
    add_dirs: Vec<String>,
    worktree: Option<Option<String>>,
    sandbox: Option<Option<String>>,
    size: Option<String>,
    json_mode: bool,
    json_schema: Option<serde_json::Value>,
    json_stream: bool,
    session_id: Option<String>,
    output_format: Option<String>,
    input_format: Option<String>,
    replay_user_messages: bool,
    include_partial_messages: bool,
    verbose: bool,
    quiet: bool,
    show_usage: bool,
    max_turns: Option<u32>,
    progress: Box<dyn ProgressHandler>,
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
            model: None,
            system_prompt: None,
            root: None,
            auto_approve: false,
            add_dirs: Vec::new(),
            worktree: None,
            sandbox: None,
            size: None,
            json_mode: false,
            json_schema: None,
            json_stream: false,
            session_id: None,
            output_format: None,
            input_format: None,
            replay_user_messages: false,
            include_partial_messages: false,
            verbose: false,
            quiet: false,
            show_usage: false,
            max_turns: None,
            progress: Box::new(SilentProgress),
        }
    }

    /// Set the provider (e.g., "claude", "codex", "gemini", "copilot", "ollama").
    pub fn provider(mut self, provider: &str) -> Self {
        self.provider = Some(provider.to_string());
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

    /// Enable streaming JSON output (NDJSON format).
    pub fn json_stream(mut self) -> Self {
        self.json_stream = true;
        self
    }

    /// Set a specific session ID (UUID).
    pub fn session_id(mut self, id: &str) -> Self {
        self.session_id = Some(id.to_string());
        self
    }

    /// Set the output format (e.g., "text", "json", "json-pretty", "stream-json").
    pub fn output_format(mut self, format: &str) -> Self {
        self.output_format = Some(format.to_string());
        self
    }

    /// Set the input format (Claude only, e.g., "text", "stream-json").
    pub fn input_format(mut self, format: &str) -> Self {
        self.input_format = Some(format.to_string());
        self
    }

    /// Re-emit user messages from stdin on stdout (Claude only).
    ///
    /// Only works with `--input-format stream-json` and `--output-format stream-json`.
    pub fn replay_user_messages(mut self, replay: bool) -> Self {
        self.replay_user_messages = replay;
        self
    }

    /// Include partial message chunks in streaming output (Claude only).
    ///
    /// Only works with `--output-format stream-json`.
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

    /// Set a custom progress handler for status reporting.
    pub fn on_progress(mut self, handler: Box<dyn ProgressHandler>) -> Self {
        self.progress = handler;
        self
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
    fn create_agent(&self, provider: &str) -> Result<Box<dyn Agent + Send + Sync>> {
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
                     Your response must conform to this JSON schema:\n{}",
                    schema_str
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
            .on_spinner_start(&format!("Initializing {} agent", provider));

        let mut agent = AgentFactory::create(
            provider,
            system_prompt,
            self.model.clone(),
            self.root.clone(),
            self.auto_approve,
            self.add_dirs.clone(),
        )?;

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
        if self.json_stream && output_format.is_none() {
            output_format = Some("stream-json".to_string());
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

        self.progress.on_spinner_finish();
        self.progress.on_success(&format!(
            "{} initialized with model {}",
            provider,
            agent.get_model()
        ));

        Ok(agent)
    }

    /// Run the agent non-interactively and return structured output.
    ///
    /// This is the primary entry point for programmatic use.
    pub async fn exec(self, prompt: &str) -> Result<AgentOutput> {
        let provider = self.resolve_provider()?;
        debug!("exec: provider={}", provider);

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

        let agent = builder.create_agent(&provider)?;

        // Handle JSON mode with prompt wrapping for non-Claude agents
        let effective_prompt = if builder.json_mode && provider != "claude" {
            let wrapped = format!(
                "IMPORTANT: You MUST respond with valid JSON only. No markdown, no explanation.\n\n{}",
                prompt
            );
            wrapped
        } else {
            prompt.to_string()
        };

        let result = agent.run(Some(&effective_prompt)).await?;

        // Clean up
        agent.cleanup().await?;

        if let Some(output) = result {
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
            Ok(output)
        } else {
            // Agent returned no structured output — create a minimal one
            Ok(AgentOutput::from_text(&provider, ""))
        }
    }

    /// Run the agent with streaming input and output (Claude only).
    ///
    /// Returns a `StreamingSession` that allows sending NDJSON messages to
    /// the agent's stdin and reading events from stdout. Automatically
    /// configures `--input-format stream-json` and `--replay-user-messages`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zag::builder::AgentBuilder;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let mut session = AgentBuilder::new()
    ///     .provider("claude")
    ///     .exec_streaming("initial prompt")
    ///     .await?;
    ///
    /// session.send_user_message("do something").await?;
    ///
    /// while let Some(event) = session.next_event().await? {
    ///     println!("{:?}", event);
    /// }
    ///
    /// session.wait().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exec_streaming(self, prompt: &str) -> Result<StreamingSession> {
        let provider = self.resolve_provider()?;
        debug!("exec_streaming: provider={}", provider);

        if provider != "claude" {
            bail!("Streaming input is only supported by the Claude provider");
        }

        let agent = self.create_agent(&provider)?;

        // Downcast to Claude to call execute_streaming
        let claude_agent = agent
            .as_any_ref()
            .downcast_ref::<Claude>()
            .ok_or_else(|| anyhow::anyhow!("Failed to downcast agent to Claude"))?;

        claude_agent.execute_streaming(Some(prompt))
    }

    /// Start an interactive agent session.
    ///
    /// This takes over stdin/stdout for the duration of the session.
    pub async fn run(self, prompt: Option<&str>) -> Result<()> {
        let provider = self.resolve_provider()?;
        debug!("run: provider={}", provider);

        let agent = self.create_agent(&provider)?;
        agent.run_interactive(prompt).await?;
        agent.cleanup().await?;
        Ok(())
    }

    /// Resume a previous session by ID.
    pub async fn resume(self, session_id: &str) -> Result<()> {
        let provider = self.resolve_provider()?;
        debug!("resume: provider={}, session={}", provider, session_id);

        let agent = self.create_agent(&provider)?;
        agent.run_resume(Some(session_id), false).await?;
        agent.cleanup().await?;
        Ok(())
    }

    /// Resume the most recent session.
    pub async fn continue_last(self) -> Result<()> {
        let provider = self.resolve_provider()?;
        debug!("continue_last: provider={}", provider);

        let agent = self.create_agent(&provider)?;
        agent.run_resume(None, true).await?;
        agent.cleanup().await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
