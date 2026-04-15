//! Streaming session for programmatic stdin/stdout interaction with agents.
//!
//! A `StreamingSession` wraps a running agent subprocess with piped stdin and
//! stdout, allowing callers to send NDJSON messages to the agent and read
//! unified events back.
//!
//! # Event lifecycle
//!
//! In bidirectional streaming mode (Claude only), [`StreamingSession::next_event`]
//! yields unified [`Event`](crate::output::Event) values converted from Claude's
//! native `stream-json` output. At the end of every agent turn the session
//! emits a [`Event::TurnComplete`](crate::output::Event::TurnComplete) with
//! the provider's `stop_reason`, a zero-based `turn_index`, and the turn's
//! `usage`, followed immediately by a per-turn
//! [`Event::Result`](crate::output::Event::Result). After that pair the
//! session remains open and accepts another
//! [`StreamingSession::send_user_message`] for the next turn. `next_event`
//! returns `Ok(None)` only when the subprocess exits (e.g. after
//! [`StreamingSession::close_input`] and EOF).
//!
//! New consumers should use `TurnComplete` as the authoritative
//! turn-boundary signal. `Result` continues to fire per-turn for backward
//! compatibility, but `TurnComplete` is what carries `stop_reason` and
//! `turn_index`. Do **not** rely on replayed `user_message` events as a
//! turn delimiter; those only appear when `--replay-user-messages` is set
//! and only fire *after* the next user message is sent.
//!
//! # Mid-turn input semantics
//!
//! `send_user_message` writes a user message to the agent's stdin. What the
//! agent does when the message arrives *while it is still producing a response
//! on the current turn* is provider-specific. Callers that need to reason about
//! mid-turn behavior should branch on
//! `ProviderCapability::features.streaming_input.semantics`, which is one of:
//!
//! - `"queue"` — the message is buffered and delivered at the next turn
//!   boundary. The current turn runs to completion; the new message becomes
//!   the next user turn. **Currently Claude.**
//! - `"interrupt"` — the message cancels the current turn and starts a new one
//!   with the new input.
//! - `"between-turns-only"` — mid-turn sends are an error or no-op; callers
//!   must wait for the current turn to finish before sending.
//!
//! Providers with `streaming_input.supported == false` (codex, gemini, copilot,
//! ollama) do not expose a `StreamingSession` at all — `exec_streaming` is
//! unavailable for them.
//!
//! # Examples
//!
//! ```no_run
//! use zag_agent::builder::AgentBuilder;
//! use zag_agent::output::Event;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut session = AgentBuilder::new()
//!     .provider("claude")
//!     .exec_streaming("initial prompt")
//!     .await?;
//!
//! // First turn: drain events until TurnComplete.
//! while let Some(event) = session.next_event().await? {
//!     println!("{:?}", event);
//!     if matches!(event, Event::TurnComplete { .. }) {
//!         break; // turn complete — the per-turn Result follows next
//!     }
//! }
//!
//! // Send a follow-up user message for the next turn.
//! session.send_user_message("do something else").await?;
//!
//! // Drain the second turn, then close the session.
//! while let Some(event) = session.next_event().await? {
//!     if matches!(event, Event::TurnComplete { .. }) {
//!         break;
//!     }
//! }
//!
//! session.close_input();
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
    /// Stateful per-event translator. Required because some unified
    /// events (e.g. `TurnComplete`) are synthesized from data carried on
    /// earlier events in the same turn.
    translator: crate::providers::claude::ClaudeEventTranslator,
    /// FIFO buffer of unified events produced by the translator but not
    /// yet returned to the caller. A single Claude `Result` expands into
    /// `[TurnComplete, Result]`, so we need room for at least two.
    pending: std::collections::VecDeque<Event>,
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
            translator: crate::providers::claude::ClaudeEventTranslator::new(),
            pending: std::collections::VecDeque::new(),
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
    ///
    /// # Mid-turn semantics
    ///
    /// The effect of calling this while the agent is still producing a
    /// response on the current turn is provider-specific. Check
    /// `ProviderCapability::features.streaming_input.semantics` at runtime
    /// to branch on behavior. The possible values are:
    ///
    /// - `"queue"` — buffered and delivered at the next turn boundary; the
    ///   current turn runs to completion. **This is Claude's behavior**, which
    ///   is the only provider currently exposing a `StreamingSession`.
    /// - `"interrupt"` — cancels the current turn and starts a new one with
    ///   the new input.
    /// - `"between-turns-only"` — mid-turn sends are an error or no-op; wait
    ///   for the current turn to finish before sending.
    ///
    /// See the module-level documentation for the full matrix.
    pub async fn send_user_message(&mut self, content: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "user_message",
            "content": content,
        });
        self.send(&serde_json::to_string(&msg)?).await
    }

    /// Read the next unified event from the agent's stdout.
    ///
    /// Lines are parsed as Claude's native `stream-json` schema and then
    /// converted into the unified [`Event`] enum. Events that don't map to a
    /// user-visible unified event (e.g. `thinking` blocks) are skipped
    /// transparently, as are blank and unparseable lines.
    ///
    /// At the end of each agent turn the session emits a
    /// [`Event::TurnComplete`](crate::output::Event::TurnComplete) followed
    /// immediately by a per-turn [`Event::Result`](crate::output::Event::Result);
    /// prefer `TurnComplete` as the turn boundary in new code. `Ok(None)`
    /// is returned only when the subprocess closes its stdout (EOF).
    pub async fn next_event(&mut self) -> Result<Option<Event>> {
        use crate::providers::claude::models::ClaudeEvent;

        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }

            match self.lines.next_line().await? {
                None => return Ok(None),
                Some(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<ClaudeEvent>(trimmed) {
                        Ok(claude_event) => {
                            for event in self.translator.translate(&claude_event) {
                                self.pending.push_back(event);
                            }
                            // Next loop iteration will drain `pending` (or
                            // read another line if the translator emitted
                            // nothing for this event).
                            continue;
                        }
                        Err(e) => {
                            log::debug!(
                                "Skipping unparseable streaming event: {}. Line: {}",
                                e,
                                crate::truncate_str(trimmed, 200)
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
                bail!("Agent process failed with status: {status}");
            } else {
                bail!("{stderr_text}");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
