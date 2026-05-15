//! Shared types and helpers for provider usage-limit detection and auto-resume.
//!
//! When a provider's CLI hits an upstream rate / usage / weekly limit, zag wants
//! to (a) detect the limit, (b) compute when it resets, (c) schedule a resume
//! attempt at that moment, and (d) record the lifecycle in the session log via
//! [`crate::session_log::LogEventKind::UsageLimitHit`] / `UsageLimitResumed` /
//! `UsageLimitResumeFailed`.
//!
//! Each provider has its own detector module (e.g.
//! `providers/claude/usage_limits.rs`). All detectors return [`UsageLimit`] so
//! the scheduler in `zag-orch` can treat the four providers uniformly.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A successful detection of an upstream usage / rate limit.
#[derive(Debug, Clone)]
pub struct UsageLimit {
    pub provider: &'static str,
    pub scope: UsageLimitScope,
    /// When usage resets, if the provider gave us something parseable.
    /// `None` means "we don't know" — `compute_resume_at` will fall back to
    /// `default_fallback_secs`.
    pub reset_at: Option<DateTime<Utc>>,
    /// The exact substring or JSON snippet that matched. Recorded into the
    /// session log so future maintainers can see why detection fired even
    /// after the upstream format has drifted.
    pub raw: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageLimitScope {
    /// Single-turn / short window (e.g. Copilot `rate_limited`).
    Session,
    /// Weekly cap (Claude `weekly`, Copilot `user_weekly_rate_limited`).
    Weekly,
    /// Account-wide global cap.
    Global,
    /// Per-day quota (Gemini `*PerDay`).
    Daily,
    /// Provider didn't surface enough info to classify.
    Unknown,
}

impl UsageLimitScope {
    pub fn as_str(self) -> &'static str {
        match self {
            UsageLimitScope::Session => "session",
            UsageLimitScope::Weekly => "weekly",
            UsageLimitScope::Global => "global",
            UsageLimitScope::Daily => "daily",
            UsageLimitScope::Unknown => "unknown",
        }
    }
}

/// Top-level `[usage_limits]` config block. Loaded from `zag.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimitConfig {
    /// Master switch. Detection always runs; this gates auto-resume scheduling.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Message injected into the session when the timer fires. Default `"Continue"`.
    #[serde(default = "default_resume_message")]
    pub resume_message: String,
    /// Hard cap on how long a single wait can be. Default 24h.
    #[serde(default = "default_max_wait_secs")]
    pub max_wait_secs: u64,
    /// Used when the provider didn't tell us a reset time. Default 1h.
    /// On self-retrigger (resume failed because limit still active), the cycle
    /// just runs again — eventually the window passes.
    #[serde(default = "default_fallback_secs")]
    pub default_fallback_secs: u64,
    /// Jitter added on top of the computed reset time, to spread retries.
    /// Default 30s.
    #[serde(default = "default_jitter_secs")]
    pub jitter_secs: u64,
    /// Maximum auto-resume attempts within a single foreground `zag exec` or
    /// `zag spawn` invocation. Default 12 — with the default 1h fallback this
    /// caps a stuck batch at ~12h. Set to 0 to disable the cap.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Per-provider overrides keyed by provider name.
    #[serde(default)]
    pub providers: HashMap<String, UsageLimitProviderOverride>,
}

impl Default for UsageLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            resume_message: default_resume_message(),
            max_wait_secs: default_max_wait_secs(),
            default_fallback_secs: default_fallback_secs(),
            jitter_secs: default_jitter_secs(),
            max_attempts: default_max_attempts(),
            providers: HashMap::new(),
        }
    }
}

/// Per-provider override. Any unset field falls back to the top-level value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageLimitProviderOverride {
    pub enabled: Option<bool>,
    pub resume_message: Option<String>,
    pub fallback_secs: Option<u64>,
    /// User-supplied regex sources OR'd into the provider's default patterns.
    /// Provider detectors compile these once via `OnceLock`.
    #[serde(default)]
    pub extra_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}
fn default_resume_message() -> String {
    "Continue".to_string()
}
fn default_max_wait_secs() -> u64 {
    86_400
}
fn default_fallback_secs() -> u64 {
    3_600
}
fn default_jitter_secs() -> u64 {
    30
}
fn default_max_attempts() -> u32 {
    12
}

impl UsageLimitConfig {
    /// True if auto-resume should be attempted for `provider`.
    pub fn enabled_for(&self, provider: &str) -> bool {
        if !self.enabled {
            return false;
        }
        self.providers
            .get(provider)
            .and_then(|o| o.enabled)
            .unwrap_or(true)
    }

    /// Effective resume message for `provider`, honoring overrides.
    pub fn resume_message_for(&self, provider: &str) -> &str {
        self.providers
            .get(provider)
            .and_then(|o| o.resume_message.as_deref())
            .unwrap_or(&self.resume_message)
    }

    /// Effective fallback duration (seconds) for `provider`.
    pub fn fallback_secs_for(&self, provider: &str) -> u64 {
        self.providers
            .get(provider)
            .and_then(|o| o.fallback_secs)
            .unwrap_or(self.default_fallback_secs)
    }

    /// User-supplied additional patterns for `provider`, empty slice if none.
    pub fn extra_patterns_for(&self, provider: &str) -> &[String] {
        self.providers
            .get(provider)
            .map(|o| o.extra_patterns.as_slice())
            .unwrap_or(&[])
    }
}

/// Compute the moment zag should attempt to resume.
///
/// Returns `(scheduled_at, fallback_used)`. `fallback_used` is true when
/// `hit.reset_at` was `None` and we substituted `fallback_secs`. The result is
/// always clamped to `now + max_wait_secs` so a malformed epoch can never
/// pin a wait into next century.
pub fn compute_resume_at(hit: &UsageLimit, cfg: &UsageLimitConfig) -> (DateTime<Utc>, bool) {
    let now = Utc::now();
    let max_wait = Duration::seconds(cfg.max_wait_secs as i64);
    let jitter = Duration::seconds(cfg.jitter_secs as i64);

    let (target, fallback_used) = match hit.reset_at {
        Some(t) => (t, false),
        None => {
            let fb = cfg.fallback_secs_for(hit.provider) as i64;
            (now + Duration::seconds(fb), true)
        }
    };

    // If the reset is in the past, clamp to "now + jitter" — gives the upstream
    // a beat to settle before we retry.
    let after_clamp = if target < now {
        now + jitter
    } else {
        target + jitter
    };

    let capped = if after_clamp > now + max_wait {
        now + max_wait
    } else {
        after_clamp
    };

    (capped, fallback_used)
}

/// Build a [`crate::session_log::LogEventKind::UsageLimitHit`] from a detected
/// `UsageLimit`. Single source of truth for the scheduled-resume path — every
/// site that knows a resume timer is going to fire (relay, foreground
/// auto-resume loop) calls this so the wire shape can never drift.
///
/// `scheduled_resume_at` and `fallback_used` come from [`compute_resume_at`].
/// `incident_id` is provided by the caller so it can be stitched into the
/// matching `UsageLimitResumed` / `UsageLimitResumeFailed` events.
pub fn log_event_hit(
    hit: &UsageLimit,
    incident_id: &str,
    scheduled_resume_at: Option<DateTime<Utc>>,
    fallback_used: bool,
) -> crate::session_log::LogEventKind {
    crate::session_log::LogEventKind::UsageLimitHit {
        provider: hit.provider.to_string(),
        scope: hit.scope.as_str().to_string(),
        reset_at: hit.reset_at.map(|t| t.to_rfc3339()),
        scheduled_resume_at: scheduled_resume_at.map(|t| t.to_rfc3339()),
        fallback_used,
        incident_id: incident_id.to_string(),
        raw: Some(hit.raw.clone()),
    }
}

/// Build a `UsageLimitHit` log event for orphan log-only detections (e.g. the
/// Codex TUI line parser) where no auto-resume scheduler is involved.
/// Generates a fresh incident id; scheduling fields are left empty because
/// nothing is scheduled.
pub fn to_log_event_hit(hit: UsageLimit) -> crate::session_log::LogEventKind {
    log_event_hit(&hit, &uuid::Uuid::new_v4().to_string(), None, false)
}

#[cfg(test)]
#[path = "usage_limits_tests.rs"]
mod tests;
