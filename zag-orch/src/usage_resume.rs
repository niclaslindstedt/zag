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
use zag_agent::agent::Agent;
use zag_agent::output::{AgentOutput, ContentBlock, Event};
use zag_agent::session_log::{LogEventKind, LogSourceKind, SessionLogWriter};
use zag_agent::usage_limits::{
    self, UsageLimit, UsageLimitConfig, UsageLimitScope, compute_resume_at,
};

use crate::usage_resume_store::{self, CompleteStatus, PendingResume};

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

/// Spawn a tokio task that sleeps until `pending.when`, then invokes
/// `strategy.resume(...)`. Emits the matching `UsageLimitResumed` or
/// `UsageLimitResumeFailed` log event so the lifecycle is reconstructable
/// from the session log alone.
///
/// Persistence: writes a `schedule` record to
/// `<state_dir>/scheduled_resumes.jsonl` before the timer is armed, and a
/// matching `complete` record after the resume call returns. The record
/// is visible via `zag usage list` while the timer is pending; cancelled
/// via `zag usage cancel <incident_id>`. If this process dies between
/// the schedule and complete records, `list_pending` still surfaces the
/// stranded resume so an operator can re-arm it manually.
///
/// The returned [`JoinHandle`] is owned by the caller (typically the relay)
/// and aborted on shutdown so a half-completed wait is not orphaned.
pub fn schedule_resume(
    pending: PendingResume,
    writer: Arc<SessionLogWriter>,
    strategy: Arc<dyn ResumeStrategy>,
) -> JoinHandle<()> {
    if let Err(e) = usage_resume_store::record_pending(pending.root.as_deref(), &pending) {
        log::warn!(
            "usage_resume: failed to persist pending resume ({}): {e}",
            pending.incident_id
        );
    }

    tokio::spawn(async move {
        let now = Utc::now();
        let wait = (pending.when - now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(0));
        log::info!(
            "usage_resume: sleeping {:?} until {} for session {} (incident {})",
            wait,
            pending.when.to_rfc3339(),
            pending.session_id,
            pending.incident_id
        );
        tokio::time::sleep(wait).await;

        match strategy
            .resume(&pending.session_id, &pending.message, pending.attempt)
            .await
        {
            Ok(()) => {
                let _ = writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::UsageLimitResumed {
                        incident_id: pending.incident_id.clone(),
                        resume_message: pending.message.clone(),
                        attempt: pending.attempt,
                    },
                );
                if let Err(e) = usage_resume_store::record_complete(
                    pending.root.as_deref(),
                    &pending.incident_id,
                    CompleteStatus::Resumed,
                    None,
                ) {
                    log::warn!(
                        "usage_resume: failed to persist complete record ({}): {e}",
                        pending.incident_id
                    );
                }
                log::info!(
                    "usage_resume: resumed session {} (incident {})",
                    pending.session_id,
                    pending.incident_id
                );
            }
            Err(e) => {
                let err_text = e.to_string();
                let _ = writer.emit(
                    LogSourceKind::Wrapper,
                    LogEventKind::UsageLimitResumeFailed {
                        incident_id: pending.incident_id.clone(),
                        error: err_text.clone(),
                        attempt: pending.attempt,
                    },
                );
                if let Err(persist_err) = usage_resume_store::record_complete(
                    pending.root.as_deref(),
                    &pending.incident_id,
                    CompleteStatus::Failed,
                    Some(&err_text),
                ) {
                    log::warn!(
                        "usage_resume: failed to persist failure record ({}): {persist_err}",
                        pending.incident_id
                    );
                }
                log::warn!(
                    "usage_resume: resume failed for session {}: {err_text}",
                    pending.session_id
                );
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Foreground auto-resume loop (used by `zag exec` and `zag spawn`, which
// spawns `zag exec` as its subprocess).
// ---------------------------------------------------------------------------

/// Run an agent with foreground auto-resume.
///
/// The agent is invoked once. If the resulting [`AgentOutput`] contains a
/// usage-limit detection (either as an explicit [`Event::UsageLimitDetected`]
/// or as text matching the provider's detector), the loop sleeps until the
/// computed resume time and re-invokes via
/// [`Agent::run_resume_with_prompt`] with the upstream provider session id
/// and the configured resume message. The loop continues until either a
/// run finishes cleanly or `cfg.max_attempts` is reached (0 = unlimited).
///
/// Each iteration's output is passed to `writer` via
/// [`zag_agent::session_log::record_agent_output`] (if `writer` is `Some`),
/// so the session log shows the full lifecycle.
///
/// Used by `zag exec` directly and by `zag spawn` indirectly (since
/// `spawn_session` spawns `zag exec` as its subprocess for non-interactive
/// mode — see `zag-orch/src/spawn.rs::build_exec_args`).
pub async fn run_with_auto_resume(
    agent: &mut (dyn Agent + Send + Sync),
    provider: &str,
    initial_prompt: String,
    initial_session_id: Option<String>,
    cfg: &UsageLimitConfig,
    writer: Option<&SessionLogWriter>,
    root: Option<&str>,
) -> Result<Option<AgentOutput>> {
    let mut current_session_id = initial_session_id;
    let mut current_prompt = initial_prompt;
    let mut attempt: u32 = 0;

    loop {
        attempt += 1;
        let output = if let Some(ref sid) = current_session_id {
            log::info!("auto_resume: invoking agent with --resume {sid} (attempt {attempt})");
            agent.run_resume_with_prompt(sid, &current_prompt).await?
        } else {
            log::info!("auto_resume: invoking agent fresh (attempt {attempt})");
            agent.run(Some(&current_prompt)).await?
        };

        if let (Some(w), Some(out)) = (writer, output.as_ref()) {
            let _ = zag_agent::session_log::record_agent_output(w, out);
        }

        // Auto-resume disabled or no structured output → return immediately.
        if !cfg.enabled_for(provider) || output.is_none() {
            return Ok(output);
        }
        let out = output.as_ref().unwrap();

        // Look for a usage-limit signal in this run's output.
        let Some(hit) = find_usage_limit_in_output(out, provider, cfg) else {
            return Ok(output);
        };

        if cfg.max_attempts > 0 && attempt >= cfg.max_attempts {
            log::warn!(
                "auto_resume: reached max attempts ({}); giving up",
                cfg.max_attempts
            );
            return Ok(output);
        }

        // Need the upstream provider session id to issue `--resume <id>` —
        // otherwise a resume would lose conversation context.
        let Some(provider_session_id) = extract_provider_session_id(out) else {
            log::warn!(
                "auto_resume: usage limit detected but no upstream session id available; cannot --resume"
            );
            return Ok(output);
        };

        let (scheduled_at, fallback_used) = compute_resume_at(&hit, cfg);
        let incident_id = uuid::Uuid::new_v4().to_string();

        // Emit a UsageLimitHit so it surfaces in `zag listen` / `zag events`,
        // even when the per-provider parser didn't produce one.
        if let Some(w) = writer {
            let _ = w.emit(
                LogSourceKind::Wrapper,
                usage_limits::log_event_hit(&hit, &incident_id, Some(scheduled_at), fallback_used),
            );
        }

        // Persist the pending resume so `zag usage list` can surface it,
        // and the next foreground attempt can recover after a crash.
        let log_path = writer.and_then(|w| w.log_path().ok()).unwrap_or_default();
        let pending = PendingResume {
            incident_id: incident_id.clone(),
            session_id: provider_session_id.clone(),
            provider: provider.to_string(),
            model: Some(agent.get_model().to_string()).filter(|s| !s.is_empty()),
            root: root.map(str::to_string),
            when: scheduled_at,
            message: cfg.resume_message_for(provider).to_string(),
            attempt,
            log_path,
        };
        if let Err(e) = usage_resume_store::record_pending(root, &pending) {
            log::warn!("auto_resume: failed to persist pending resume: {e}");
        }

        let wait = (scheduled_at - Utc::now())
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(0));
        log::info!(
            "auto_resume: waiting {:?} until {} before resume (incident {}, attempt {})",
            wait,
            scheduled_at.to_rfc3339(),
            incident_id,
            attempt
        );
        tokio::time::sleep(wait).await;

        if let Some(w) = writer {
            let _ = w.emit(
                LogSourceKind::Wrapper,
                LogEventKind::UsageLimitResumed {
                    incident_id: incident_id.clone(),
                    resume_message: cfg.resume_message_for(provider).to_string(),
                    attempt,
                },
            );
        }
        if let Err(e) =
            usage_resume_store::record_complete(root, &incident_id, CompleteStatus::Resumed, None)
        {
            log::warn!("auto_resume: failed to persist complete record: {e}");
        }

        current_session_id = Some(provider_session_id);
        current_prompt = cfg.resume_message_for(provider).to_string();
    }
}

/// Pull the upstream provider session id out of an [`AgentOutput`].
///
/// Prefers `output.session_id` (populated by some providers like Codex), then
/// falls back to scanning `Event::Init.metadata.session_id` (the path Claude
/// uses) or the existing `provider_session_id` recorded on assistant events.
pub fn extract_provider_session_id(output: &AgentOutput) -> Option<String> {
    if !output.session_id.is_empty() && output.session_id != "unknown" {
        return Some(output.session_id.clone());
    }
    output.events.iter().find_map(|e| match e {
        Event::Init { metadata, .. } => metadata
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        _ => None,
    })
}

/// Look for a usage-limit signal in an [`AgentOutput`].
///
/// Order:
/// 1. An explicit [`Event::UsageLimitDetected`] in `output.events` (emitted by
///    the streaming/batch translator when wired).
/// 2. Fall back to text-scanning event content with the provider's own
///    detector — useful for providers whose batch path doesn't run the
///    detector yet (Codex/Copilot/Gemini exec mode).
pub fn find_usage_limit_in_output(
    output: &AgentOutput,
    provider: &str,
    cfg: &UsageLimitConfig,
) -> Option<UsageLimit> {
    for event in &output.events {
        if let Event::UsageLimitDetected {
            provider: ev_provider,
            scope,
            reset_at,
            raw,
        } = event
        {
            return Some(UsageLimit {
                provider: provider_static_str(ev_provider),
                scope: scope_from_str(scope),
                reset_at: reset_at
                    .as_deref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                raw: raw.clone().unwrap_or_default(),
            });
        }
    }

    let blob = extract_text_blob(output);
    if blob.is_empty() {
        return None;
    }
    match provider {
        "claude" => zag_agent::providers::claude::usage_limits::detect_text(&blob, cfg),
        "codex" => zag_agent::providers::codex_usage_limits::detect_text(&blob, cfg),
        "copilot" => zag_agent::providers::copilot_usage_limits::detect_text(&blob, cfg),
        "gemini" => zag_agent::providers::gemini_usage_limits::detect_text(&blob, cfg),
        _ => None,
    }
}

fn extract_text_blob(output: &AgentOutput) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(r) = &output.result {
        parts.push(r.clone());
    }
    if let Some(em) = &output.error_message {
        parts.push(em.clone());
    }
    for event in &output.events {
        match event {
            Event::AssistantMessage { content, .. } => {
                for block in content {
                    if let ContentBlock::Text { text } = block {
                        parts.push(text.clone());
                    }
                }
            }
            Event::Result {
                message: Some(m), ..
            } => parts.push(m.clone()),
            Event::Error { message, .. } => parts.push(message.clone()),
            _ => {}
        }
    }
    parts.join("\n")
}

fn provider_static_str(provider: &str) -> &'static str {
    match provider {
        "claude" => "claude",
        "codex" => "codex",
        "copilot" => "copilot",
        "gemini" => "gemini",
        _ => "unknown",
    }
}

fn scope_from_str(s: &str) -> UsageLimitScope {
    match s {
        "session" => UsageLimitScope::Session,
        "weekly" => UsageLimitScope::Weekly,
        "global" => UsageLimitScope::Global,
        "daily" => UsageLimitScope::Daily,
        _ => UsageLimitScope::Unknown,
    }
}

#[cfg(test)]
#[path = "usage_resume_tests.rs"]
mod tests;
