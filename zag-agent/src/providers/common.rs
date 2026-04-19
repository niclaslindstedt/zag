use crate::agent::OnSpawnHook;
use crate::output::AgentOutput;
use crate::sandbox::SandboxConfig;
use anyhow::Context;
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Shared configuration state for CLI-based agent providers.
///
/// Embed this struct in each provider to avoid duplicating field
/// declarations and trivial setter implementations.
pub struct CommonAgentState {
    pub system_prompt: String,
    pub model: String,
    pub root: Option<String>,
    pub skip_permissions: bool,
    pub output_format: Option<String>,
    pub add_dirs: Vec<String>,
    pub capture_output: bool,
    pub sandbox: Option<SandboxConfig>,
    pub max_turns: Option<u32>,
    pub env_vars: Vec<(String, String)>,
    /// Optional callback invoked with the OS pid of the spawned agent
    /// subprocess. Threaded through from `AgentBuilder::on_spawn` /
    /// `Agent::set_on_spawn_hook`. Providers call
    /// [`CommonAgentState::notify_spawn`] right after `Command::spawn`
    /// so callers can capture the child pid before the terminal wait.
    pub on_spawn_hook: Option<OnSpawnHook>,
}

impl CommonAgentState {
    pub fn new(default_model: &str) -> Self {
        Self {
            system_prompt: String::new(),
            model: default_model.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
            add_dirs: Vec::new(),
            capture_output: false,
            sandbox: None,
            max_turns: None,
            env_vars: Vec::new(),
            on_spawn_hook: None,
        }
    }

    /// Invoke the registered `on_spawn` hook with the pid of a freshly
    /// spawned child, if any. Safe to call even when no hook is set.
    pub fn notify_spawn(&self, child: &Child) {
        if let (Some(cb), Some(pid)) = (self.on_spawn_hook.as_ref(), child.id()) {
            cb(pid);
        }
    }

    /// Get the effective base path (root directory or ".").
    pub fn get_base_path(&self) -> &Path {
        self.root.as_ref().map(Path::new).unwrap_or(Path::new("."))
    }

    /// Create a `Command` either directly or wrapped in sandbox.
    ///
    /// Standard pattern used by Claude, Copilot, and Gemini. Sets
    /// `current_dir`, args, and env vars. Providers with custom sandbox
    /// behavior (Codex, Ollama) keep their own `make_command()`.
    pub fn make_command(&self, binary_name: &str, agent_args: Vec<String>) -> Command {
        if let Some(ref sb) = self.sandbox {
            let std_cmd = crate::sandbox::build_sandbox_command(sb, agent_args);
            Command::from(std_cmd)
        } else {
            let mut cmd = Command::new(binary_name);
            if let Some(ref root) = self.root {
                cmd.current_dir(root);
            }
            cmd.args(&agent_args);
            for (key, value) in &self.env_vars {
                cmd.env(key, value);
            }
            cmd
        }
    }

    /// Execute a command interactively (inheriting stdin/stdout/stderr).
    ///
    /// Returns `ProcessError` on non-zero exit.
    pub async fn run_interactive_command(
        cmd: &mut Command,
        agent_display_name: &str,
    ) -> anyhow::Result<()> {
        Self::run_interactive_command_with_hook(cmd, agent_display_name, None).await
    }

    /// Same as [`run_interactive_command`], but invokes `on_spawn` once
    /// with the child's OS pid right after spawn and before awaiting
    /// the child's exit — that window is what lets callers register the
    /// child pid with an external process store (e.g. so
    /// `zag ps kill self` can SIGTERM the agent child rather than the
    /// parent zag process).
    pub async fn run_interactive_command_with_hook(
        cmd: &mut Command,
        agent_display_name: &str,
        on_spawn: Option<&OnSpawnHook>,
    ) -> anyhow::Result<()> {
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to execute '{}' CLI. Is it installed and in PATH?",
                agent_display_name.to_lowercase()
            )
        })?;
        if let (Some(cb), Some(pid)) = (on_spawn, child.id()) {
            cb(pid);
        }
        let status = child.wait().await.with_context(|| {
            format!(
                "Failed waiting on '{}' CLI",
                agent_display_name.to_lowercase()
            )
        })?;
        if !status.success() {
            return Err(crate::process::ProcessError {
                exit_code: status.code(),
                stderr: String::new(),
                agent_name: agent_display_name.to_string(),
            }
            .into());
        }
        Ok(())
    }

    /// Execute a non-interactive command with simple capture-or-passthrough.
    ///
    /// If `capture_output` is set, captures stdout and returns `Some(AgentOutput)`.
    /// Otherwise streams stdout to the terminal and returns `None`.
    ///
    /// Used by Copilot, Gemini, Ollama. Providers with custom output parsing
    /// (Claude, Codex) keep their own non-interactive logic.
    pub async fn run_non_interactive_simple(
        &self,
        cmd: &mut Command,
        agent_display_name: &str,
    ) -> anyhow::Result<Option<AgentOutput>> {
        if self.capture_output {
            let text = crate::process::run_captured(cmd, agent_display_name).await?;
            log::debug!(
                "{} raw response ({} bytes): {}",
                agent_display_name,
                text.len(),
                text
            );
            Ok(Some(AgentOutput::from_text(
                &agent_display_name.to_lowercase(),
                &text,
            )))
        } else {
            cmd.stdin(Stdio::inherit()).stdout(Stdio::inherit());
            crate::process::run_with_captured_stderr(cmd).await?;
            Ok(None)
        }
    }
}

/// Delegate common Agent trait setter methods to `self.common`.
///
/// Generates the 12 trivial setters that are identical across all CLI-based
/// providers. Excludes `set_skip_permissions` since Ollama overrides it.
macro_rules! impl_common_agent_setters {
    () => {
        fn system_prompt(&self) -> &str {
            &self.common.system_prompt
        }

        fn set_system_prompt(&mut self, prompt: String) {
            self.common.system_prompt = prompt;
        }

        fn get_model(&self) -> &str {
            &self.common.model
        }

        fn set_model(&mut self, model: String) {
            self.common.model = model;
        }

        fn set_root(&mut self, root: String) {
            self.common.root = Some(root);
        }

        fn set_output_format(&mut self, format: Option<String>) {
            self.common.output_format = format;
        }

        fn set_add_dirs(&mut self, dirs: Vec<String>) {
            self.common.add_dirs = dirs;
        }

        fn set_env_vars(&mut self, vars: Vec<(String, String)>) {
            self.common.env_vars = vars;
        }

        fn set_capture_output(&mut self, capture: bool) {
            self.common.capture_output = capture;
        }

        fn set_sandbox(&mut self, config: crate::sandbox::SandboxConfig) {
            self.common.sandbox = Some(config);
        }

        fn set_max_turns(&mut self, turns: u32) {
            self.common.max_turns = Some(turns);
        }

        fn set_on_spawn_hook(&mut self, hook: crate::agent::OnSpawnHook) {
            self.common.on_spawn_hook = Some(hook);
        }
    };
}
pub(crate) use impl_common_agent_setters;

#[cfg(test)]
mod on_spawn_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// `run_interactive_command` must spawn the child, invoke the
    /// `on_spawn` callback with the real child pid, and then wait for
    /// the child to exit. The pid surfaced to the callback must match
    /// the actual OS pid observed by the spawn (proved by checking
    /// that it is non-zero — on Unix, a freshly spawned child always
    /// has a live pid).
    #[tokio::test]
    async fn notify_spawn_delivers_pid_before_wait() {
        let captured = Arc::new(AtomicU32::new(0));
        let captured_clone = captured.clone();
        let hook: OnSpawnHook = Arc::new(move |pid| {
            captured_clone.store(pid, Ordering::SeqCst);
        });

        let mut cmd = Command::new("true");
        CommonAgentState::run_interactive_command_with_hook(&mut cmd, "Test", Some(&hook))
            .await
            .expect("`true` must exit 0");

        let pid = captured.load(Ordering::SeqCst);
        assert!(pid > 0, "expected a non-zero child pid, got {pid}");
    }

    #[tokio::test]
    async fn notify_spawn_without_hook_is_noop() {
        // Sanity check: the helper still works when no hook is passed.
        let mut cmd = Command::new("true");
        CommonAgentState::run_interactive_command_with_hook(&mut cmd, "Test", None)
            .await
            .expect("`true` must exit 0");
    }

    /// The original 2-arg signature is preserved as a backwards-compat
    /// shim so downstream consumers of the public `CommonAgentState`
    /// API keep compiling without passing a hook.
    #[tokio::test]
    async fn legacy_two_arg_signature_still_works() {
        let mut cmd = Command::new("true");
        CommonAgentState::run_interactive_command(&mut cmd, "Test")
            .await
            .expect("`true` must exit 0");
    }
}

/// Implement `as_any_ref` and `as_any_mut` for a concrete agent type.
macro_rules! impl_as_any {
    () => {
        fn as_any_ref(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}
pub(crate) use impl_as_any;
