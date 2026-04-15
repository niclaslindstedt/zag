use anyhow::Result;
use log::debug;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;

/// Structured error from a failed subprocess.
///
/// Wraps the exit code and stderr so callers can inspect them
/// (e.g. to populate `AgentOutput.exit_code` / `error_message`).
#[derive(Debug, Clone)]
pub struct ProcessError {
    /// The process exit code, if available.
    pub exit_code: Option<i32>,
    /// Captured stderr text (may be empty).
    pub stderr: String,
    /// Name of the agent / command that failed.
    pub agent_name: String,
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.stderr.is_empty() {
            write!(
                f,
                "{} command failed with exit code {:?}",
                self.agent_name, self.exit_code
            )
        } else {
            write!(f, "{}", self.stderr)
        }
    }
}

impl std::error::Error for ProcessError {}

/// Read stderr from a child process handle into a trimmed String.
async fn read_stderr(handle: Option<tokio::process::ChildStderr>) -> String {
    if let Some(stderr) = handle {
        let mut buf = Vec::new();
        let mut reader = tokio::io::BufReader::new(stderr);
        let _ = reader.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).trim().to_string()
    } else {
        String::new()
    }
}

/// Log non-empty stderr text.
pub fn log_stderr_text(stderr: &str) {
    if !stderr.is_empty() {
        for line in stderr.lines() {
            debug!("[STDERR] {line}");
        }
    }
}

/// Log stderr and bail on non-zero exit status.
///
/// Returns `Ok(())` on success. On failure, logs stderr to file and
/// returns an error containing the stderr text (or the exit status if stderr is empty).
pub fn check_exit_status(
    status: std::process::ExitStatus,
    stderr: &str,
    agent_name: &str,
) -> Result<()> {
    debug!("{agent_name} process exited with status: {status}");
    if status.success() {
        return Ok(());
    }
    Err(ProcessError {
        exit_code: status.code(),
        stderr: stderr.to_string(),
        agent_name: agent_name.to_string(),
    }
    .into())
}

/// Handle stderr logging and exit status checking for a completed `Output`.
///
/// Logs any stderr to file, then bails if exit status is non-zero.
pub fn handle_output(output: &std::process::Output, agent_name: &str) -> Result<()> {
    let stderr_text = String::from_utf8_lossy(&output.stderr);
    let stderr_text = stderr_text.trim();
    log_stderr_text(stderr_text);
    check_exit_status(output.status, stderr_text, agent_name)
}

/// Run a command capturing stdout and stderr, returning stdout text on success.
///
/// Stdin is inherited. On failure, stderr is included in the error message.
pub async fn run_captured(cmd: &mut Command, agent_name: &str) -> Result<String> {
    debug!("{agent_name}: running with captured stdout/stderr");
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().await?;
    debug!(
        "{}: captured {} bytes stdout, {} bytes stderr",
        agent_name,
        output.stdout.len(),
        output.stderr.len()
    );
    handle_output(&output, agent_name)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a command with stderr captured.
///
/// - On success (exit code 0): captured stderr is logged to file only
/// - On failure (exit code != 0): captured stderr is logged to file AND returned in the error
///
/// Stdout and stdin should be configured by the caller before calling this function.
/// This function only sets stderr to piped.
pub async fn run_with_captured_stderr(cmd: &mut Command) -> Result<()> {
    debug!("Running command with captured stderr");
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stderr_handle = child.stderr.take();
    let status = child.wait().await?;
    let stderr_text = read_stderr(stderr_handle).await;

    log_stderr_text(&stderr_text);
    check_exit_status(status, &stderr_text, "Command")
}

/// Spawn a command with stderr captured, but stdout piped for reading.
///
/// Returns the child process. The caller is responsible for reading stdout
/// and calling `wait_with_stderr()` when done.
pub async fn spawn_with_captured_stderr(cmd: &mut Command) -> Result<tokio::process::Child> {
    debug!("Spawning command with captured stderr");
    cmd.stderr(Stdio::piped());
    let child = cmd.spawn()?;
    Ok(child)
}

/// Wait for a child process and handle its captured stderr.
///
/// - On success: stderr logged to file only
/// - On failure: stderr logged to file AND returned in the error
pub async fn wait_with_stderr(mut child: tokio::process::Child) -> Result<()> {
    let stderr_handle = child.stderr.take();
    let status = child.wait().await?;
    let stderr_text = read_stderr(stderr_handle).await;

    log_stderr_text(&stderr_text);
    check_exit_status(status, &stderr_text, "Command")
}
