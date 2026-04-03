//! Streaming session for programmatic stdin/stdout interaction with agents.
//!
//! A `StreamingSession` wraps a running agent subprocess with piped stdin and
//! stdout, allowing callers to send NDJSON messages to the agent and read
//! unified events back.
//!
//! # Examples
//!
//! ```no_run
//! use zag::builder::AgentBuilder;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut session = AgentBuilder::new()
//!     .provider("claude")
//!     .exec_streaming("initial prompt")
//!     .await?;
//!
//! // Send a user message
//! session.send_user_message("do something").await?;
//!
//! // Read events
//! while let Some(event) = session.next_event().await? {
//!     println!("{:?}", event);
//! }
//!
//! session.wait().await?;
//! # Ok(())
//! # }
//! ```

use crate::output::Event;
use anyhow::{Result, bail};
use serde_json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// A live streaming session connected to an agent subprocess.
///
/// stdin is piped for sending NDJSON messages, stdout is piped for reading
/// NDJSON events. The session owns the child process.
pub struct StreamingSession {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Lines<BufReader<ChildStdout>>,
}

impl StreamingSession {
    /// Create a new `StreamingSession` from a spawned child process.
    ///
    /// The child must have been spawned with piped stdin and stdout.
    pub(crate) fn new(mut child: Child) -> Result<Self> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Child process stdout not piped"))?;
        let stdin = child.stdin.take();
        let reader = BufReader::new(stdout);
        let lines = reader.lines();

        Ok(Self {
            child,
            stdin,
            lines,
        })
    }

    /// Send a raw NDJSON line to the agent's stdin.
    ///
    /// The message should be a single JSON object (no trailing newline needed).
    pub async fn send(&mut self, message: &str) -> Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("stdin already closed"))?;
        stdin.write_all(message.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Send a user message to the agent.
    ///
    /// Formats the content as a `{"type":"user_message","content":"..."}` NDJSON line.
    pub async fn send_user_message(&mut self, content: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "user_message",
            "content": content,
        });
        self.send(&serde_json::to_string(&msg)?).await
    }

    /// Read the next event from the agent's stdout.
    ///
    /// Returns `None` when stdout is closed (agent exited).
    /// Skips lines that fail to parse as JSON events.
    pub async fn next_event(&mut self) -> Result<Option<Event>> {
        loop {
            match self.lines.next_line().await? {
                None => return Ok(None),
                Some(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<Event>(trimmed) {
                        Ok(event) => return Ok(Some(event)),
                        Err(e) => {
                            log::debug!(
                                "Skipping unparseable streaming event: {}. Line: {}",
                                e,
                                &trimmed[..trimmed.len().min(200)]
                            );
                            continue;
                        }
                    }
                }
            }
        }
    }

    /// Close the stdin pipe, signaling no more input to the agent.
    pub fn close_input(&mut self) {
        self.stdin.take();
    }

    /// Wait for the agent process to exit.
    ///
    /// Consumes the session. Returns an error if the process exits with a
    /// non-zero status.
    pub async fn wait(mut self) -> Result<()> {
        // Drop stdin to ensure the agent sees EOF
        self.stdin.take();

        let stderr_handle = self.child.stderr.take();
        let status = self.child.wait().await?;

        let stderr_text = if let Some(stderr) = stderr_handle {
            let mut buf = Vec::new();
            let mut reader = tokio::io::BufReader::new(stderr);
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buf).await;
            String::from_utf8_lossy(&buf).trim().to_string()
        } else {
            String::new()
        };

        crate::process::log_stderr_text(&stderr_text);

        if !status.success() {
            if stderr_text.is_empty() {
                bail!("Agent process failed with status: {}", status);
            } else {
                bail!("{}", stderr_text);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
