use crate::logging;
use anyhow::Result;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

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

    let stderr_text = if let Some(stderr) = stderr_handle {
        let mut buf = Vec::new();
        let mut reader = tokio::io::BufReader::new(stderr);
        let _ = reader.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).trim().to_string()
    } else {
        String::new()
    };

    if !stderr_text.is_empty() {
        for line in stderr_text.lines() {
            logging::log_to_file(&format!("[STDERR] {}", line));
        }
    }

    if !status.success() {
        if stderr_text.is_empty() {
            anyhow::bail!("Command failed with status: {}", status);
        } else {
            anyhow::bail!("{}", stderr_text);
        }
    }

    Ok(())
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

    let stderr_text = if let Some(stderr) = stderr_handle {
        let mut buf = Vec::new();
        let mut reader = tokio::io::BufReader::new(stderr);
        let _ = reader.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).trim().to_string()
    } else {
        String::new()
    };

    if !stderr_text.is_empty() {
        for line in stderr_text.lines() {
            logging::log_to_file(&format!("[STDERR] {}", line));
        }
    }

    if !status.success() {
        if stderr_text.is_empty() {
            anyhow::bail!("Command failed with status: {}", status);
        } else {
            anyhow::bail!("{}", stderr_text);
        }
    }

    Ok(())
}
