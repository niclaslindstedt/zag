//! Claude usage-limit detection.
//!
//! This is the *single* file maintainers touch when the Claude CLI changes its
//! usage-limit output format. Patterns are at the top of the file, fixture
//! tests cover real-world captures, and the public API hands back a uniform
//! [`UsageLimit`] for the orch scheduler to consume.
//!
//! Claude emits a clean machine-readable line:
//!
//! ```text
//! Claude AI usage limit reached|<unix_epoch>
//! Claude AI weekly usage limit reached|<unix_epoch>
//! Claude AI global usage limit reached|<unix_epoch>
//! ```
//!
//! This appears as the assistant message text in the stream-json output once
//! the upstream limit is hit. We also recognize the `system/api_retry` and
//! `result/subtype:"error"` envelopes that carry `rate_limit` / `error_status:
//! 429` — those reset times are not available, so detection there falls back
//! to the configurable wait.

use crate::usage_limits::{UsageLimit, UsageLimitConfig, UsageLimitScope};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;

/// Default patterns. Order matters — first match wins.
///
/// **Maintainer note:** when Claude changes the limit string, add the new
/// pattern here and a captured fixture under
/// `zag-agent/tests/fixtures/usage_limits/claude/`. Old patterns can stay so
/// older session logs still parse.
pub const DEFAULT_PATTERNS: &[&str] = &[
    // Scoped variant: "Claude AI weekly usage limit reached|1760000400"
    r"Claude AI (?P<scope>weekly|global) usage limit reached\|(?P<epoch>\d+)",
    // Bare variant: "Claude AI usage limit reached|1760000400"
    r"Claude AI usage limit reached\|(?P<epoch>\d+)",
];

static COMPILED: OnceLock<Vec<Regex>> = OnceLock::new();

fn compiled_defaults() -> &'static [Regex] {
    COMPILED.get_or_init(|| {
        DEFAULT_PATTERNS
            .iter()
            .map(|src| Regex::new(src).expect("Claude usage-limit default pattern is valid regex"))
            .collect()
    })
}

fn compile_extras(extras: &[String]) -> Vec<Regex> {
    extras
        .iter()
        .filter_map(|src| match Regex::new(src) {
            Ok(r) => Some(r),
            Err(e) => {
                log::warn!("Ignoring invalid claude usage-limit pattern {src:?}: {e}");
                None
            }
        })
        .collect()
}

fn scope_from_capture(cap: &regex::Captures) -> UsageLimitScope {
    cap.name("scope")
        .map(|m| match m.as_str() {
            "weekly" => UsageLimitScope::Weekly,
            "global" => UsageLimitScope::Global,
            _ => UsageLimitScope::Session,
        })
        .unwrap_or(UsageLimitScope::Session)
}

fn reset_from_capture(cap: &regex::Captures) -> Option<DateTime<Utc>> {
    let epoch_str = cap.name("epoch")?.as_str();
    let epoch: i64 = epoch_str.parse().ok()?;
    DateTime::from_timestamp(epoch, 0)
}

/// Scan a single text line for any of the known Claude usage-limit patterns.
///
/// Returns `Some(UsageLimit)` on the first match. Provider overrides extend
/// the default pattern list — they don't replace it.
pub fn detect_text(line: &str, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("claude") {
        return None;
    }

    for re in compiled_defaults() {
        if let Some(cap) = re.captures(line) {
            return Some(UsageLimit {
                provider: "claude",
                scope: scope_from_capture(&cap),
                reset_at: reset_from_capture(&cap),
                raw: cap
                    .get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }
    }

    for re in compile_extras(cfg.extra_patterns_for("claude")) {
        if let Some(cap) = re.captures(line) {
            return Some(UsageLimit {
                provider: "claude",
                scope: scope_from_capture(&cap),
                reset_at: reset_from_capture(&cap),
                raw: cap
                    .get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }
    }

    None
}

/// Scan a Claude stream-json event JSON value for usage-limit signals.
///
/// Covers two cases:
/// 1. `system / api_retry` event with `error_status: 429` or `error:
///    "rate_limit"` — no reset time available; falls back to config.
/// 2. `result / subtype: "error"` with `error: "rate_limit"` — same.
///
/// The primary `Claude AI usage limit reached|<epoch>` line is detected via
/// `detect_text` against assistant message text — this `detect_json` path
/// catches the structured retry/error envelopes that don't carry the line.
pub fn detect_json(value: &serde_json::Value, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("claude") {
        return None;
    }

    let kind = value.get("type").and_then(|v| v.as_str())?;
    let subtype = value.get("subtype").and_then(|v| v.as_str());
    let error = value.get("error").and_then(|v| v.as_str());
    let error_status = value
        .get("error_status")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let is_rate_limit = error == Some("rate_limit") || error_status == 429;

    let matched = match (kind, subtype) {
        ("system", Some("api_retry")) if is_rate_limit => true,
        ("result", Some("error")) if is_rate_limit => true,
        _ => false,
    };

    if !matched {
        return None;
    }

    Some(UsageLimit {
        provider: "claude",
        scope: UsageLimitScope::Session,
        reset_at: None, // structured envelope doesn't carry one
        raw: value.to_string(),
    })
}

#[cfg(test)]
#[path = "usage_limits_tests.rs"]
mod tests;
