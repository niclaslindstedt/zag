//! Scheduled auto-resume after a detected usage limit.
//!
//! When a provider's parser emits a [`LogEventKind::UsageLimitHit`], the relay
//! arms a wake-up timer here. The timer eventually fires
//! [`ResumeStrategy::resume`], which delivers the configured resume message
//! (default `"Continue"`) into the live session via whichever path makes
//! sense for that provider. Two strategies ship in this crate:
//!
//! - [`FifoResumeStrategy`] — Claude. Writes a `user_message` NDJSON line
//!   into the relay's FIFO, where the relay loop picks it up and feeds it to
//!   the bidirectional streaming session as if a human typed it.
//! - [`RespawnResumeStrategy`] — Codex, Copilot, Gemini. Calls
//!   `agent.run_resume_with_prompt(provider_session_id, message)` to launch
//!   a fresh `--resume <id>` invocation of the upstream CLI.
//!
//! Either way, the lifecycle (`UsageLimitResumed` or
//! `UsageLimitResumeFailed`) is emitted to the same session log so users see
//! the timeline via `zag listen` / `zag events`.
//!
//! See `docs/usage-limits.md` for the full feature documentation.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::task::JoinHandle;
use zag_agent::session_log::{LogEventKind, LogSourceKind, SessionLogWriter};

/// Pluggable delivery mechanism for a scheduled resume.
///
/// One implementation per provider class. The trait is async via boxed
/// futures so it stays object-safe — the relay stores `Arc<dyn ResumeStrategy>`.
pub trait ResumeStrategy: Send + Sync {
    /// Deliver `message` to `session_id`. `attempt` increments only within a
    /// single relay process (across respawns each is a fresh attempt-1).
    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        message: &'a str,
        attempt: u32,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

/// Resume by writing into the interactive relay's FIFO. Used for Claude where
/// the bidirectional `stream-json` session is still alive when the timer fires.
pub struct FifoResumeStrategy;

impl ResumeStrategy for FifoResumeStrategy {
    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        message: &'a str,
        _attempt: u32,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { crate::messaging::send_via_fifo(session_id, message).await })
    }
}

/// Resume by re-invoking the provider with `--resume <provider_session_id>` and
/// `message` as the new prompt. Used for non-Claude providers whose upstream
/// process has already exited by the time the timer fires.
///
/// Currently relies on each provider's `Agent::run_resume_with_prompt` impl.
/// Codex implements it; Copilot and Gemini do not (yet). When the trait method
/// is missing, this strategy surfaces a clear error via the session log so the
/// user knows manual resume is needed.
pub struct RespawnResumeStrategy {
    pub provider: String,
    pub model: Option<String>,
    pub root: Option<String>,
}

impl ResumeStrategy for RespawnResumeStrategy {
    fn resume<'a>(
        &'a self,
        session_id: &'a str,
        message: &'a str,
        _attempt: u32,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        let provider = self.provider.clone();
        let model = self.model.clone();
        let root = self.root.clone();
        let session_id = session_id.to_string();
        let message = message.to_string();
        Box::pin(async move {
            // Look up the upstream session id (each provider's own session id),
            // which is what `--resume` takes — not zag's wrapper id.
            let store = zag_agent::session::SessionStore::load(root.as_deref()).unwrap_or_default();
            let entry = store
                .find_by_any_id(&session_id)
                .ok_or_else(|| anyhow::anyhow!("Session {session_id} not found in store"))?;
            let provider_session_id = entry.provider_session_id.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "Session {session_id} has no upstream provider_session_id; cannot --resume"
                )
            })?;

            let agent = zag_agent::factory::AgentFactory::create(
                &provider,
                None,
                model,
                root,
                false,
                Vec::new(),
            )?;
            agent
                .run_resume_with_prompt(&provider_session_id, &message)
                .await?;
            Ok(())
        })
    }
}

/// Pick the right strategy for a provider. Caller is responsible for
/// disabling resume entirely when `cfg.enabled_for(provider)` is false.
pub fn strategy_for(
    provider: &str,
    model: Option<String>,
    root: Option<String>,
) -> Arc<dyn ResumeStrategy> {
    match provider {
        "claude" => Arc::new(FifoResumeStrategy),
        _ => Arc::new(RespawnResumeStrategy {
            provider: provider.to_string(),
            model,
            root,
        }),
    }
}

/// Spawn a tokio task that sleeps until `when`, then invokes
/// `strategy.resume(session_id, message)`. Emits the matching `UsageLimitResumed`
/// or `UsageLimitResumeFailed` log event so the lifecycle is reconstructable
/// from the session log alone.
///
/// The returned [`JoinHandle`] is owned by the caller (typically the relay)
/// and aborted on shutdown so a half-completed wait is not orphaned.
pub fn schedule_resume(
    session_id: String,
    when: DateTime<Utc>,
    message: String,
    incident_id: String,
    attempt: u32,
    writer: Arc<SessionLogWriter>,
    strategy: Arc<dyn ResumeStrategy>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let now = Utc::now();
        let wait = (when - now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(0));
        log::info!(
            "usage_resume: sleeping {:?} until {} for session {} (incident {})",
            wait,
            when.to_rfc3339(),
            session_id,
            incident_id
        );
        tokio::time::sleep(wait).await;

        match strategy.resume(&session_id, &message, attempt).await {
            Ok(()) => {
                let _ = writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::UsageLimitResumed {
                        incident_id: incident_id.clone(),
                        resume_message: message.clone(),
                        attempt,
                    },
                );
                log::info!("usage_resume: resumed session {session_id} (incident {incident_id})");
            }
            Err(e) => {
                let _ = writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::UsageLimitResumeFailed {
                        incident_id: incident_id.clone(),
                        error: e.to_string(),
                        attempt,
                    },
                );
                log::warn!("usage_resume: resume failed for session {session_id}: {e}");
            }
        }
    })
}

#[cfg(test)]
#[path = "usage_resume_tests.rs"]
mod tests;
