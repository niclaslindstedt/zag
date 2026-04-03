use anyhow::{Context, Result};
use log::debug;
use std::process::Command;

/// Configuration for running an agent inside a Docker sandbox.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub name: String,
    pub template: String,
    pub workspace: String,
}

/// Return the Docker sandbox template image for a given provider.
pub fn template_for_provider(provider: &str) -> &str {
    match provider {
        "claude" => "docker/sandbox-templates:claude-code",
        "codex" => "docker/sandbox-templates:codex",
        "gemini" => "docker/sandbox-templates:gemini",
        "copilot" => "docker/sandbox-templates:copilot",
        "ollama" => "shell",
        _ => "docker/sandbox-templates:claude-code",
    }
}

/// Generate a random sandbox name like `sandbox-a1b2c3d4`.
pub fn generate_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let hash = seed ^ (std::process::id() as u128);
    format!("sandbox-{:08x}", (hash & 0xFFFF_FFFF) as u32)
}

/// Build a `Command` that runs agent args inside a Docker sandbox.
///
/// Produces: `docker sandbox run --name <name> <template> <workspace> -- <agent_args...>`
pub fn build_sandbox_command(config: &SandboxConfig, agent_args: Vec<String>) -> Command {
    let mut cmd = Command::new("docker");
    cmd.args([
        "sandbox",
        "run",
        "--name",
        &config.name,
        &config.template,
        &config.workspace,
        "--",
    ]);
    cmd.args(&agent_args);
    debug!(
        "Sandbox command: docker sandbox run --name {} {} {} -- {}",
        config.name,
        config.template,
        config.workspace,
        agent_args.join(" ")
    );
    cmd
}

/// Remove a Docker sandbox by name.
pub fn remove_sandbox(name: &str) -> Result<()> {
    debug!("Removing sandbox: {}", name);
    let output = Command::new("docker")
        .args(["sandbox", "rm", name])
        .output()
        .context("Failed to run docker sandbox rm")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to remove sandbox: {}", stderr.trim());
    }
    debug!("Sandbox removed: {}", name);
    Ok(())
}

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod tests;
