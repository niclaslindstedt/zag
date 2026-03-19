use crate::logging;
use anyhow::Result;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;

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

/// Log non-empty stderr text to the session log file.
pub fn log_stderr_text(stderr: &str) {
    if !stderr.is_empty() {
        for line in stderr.lines() {
            logging::log_to_file(&format!("[STDERR] {}", line));
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
    if status.success() {
        return Ok(());
    }
    if stderr.is_empty() {
        anyhow::bail!("{} command failed with status: {}", agent_name, status);
    } else {
        anyhow::bail!("{}", stderr);
    }
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
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output().await?;
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
