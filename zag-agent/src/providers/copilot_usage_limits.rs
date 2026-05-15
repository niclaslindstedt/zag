//! Copilot usage-limit detection.
//!
//! Single file to update when the Copilot CLI changes its limit output.
//!
//! Copilot writes events to a per-session `events.jsonl` file. On rate-limit it
//! emits an error event whose body contains a structured error object:
//!
//! ```json
//! {
//!   "error": {
//!     "message": "Sorry, you've hit a rate limit ... Please try again in 2 hours.",
//!     "code": "rate_limited"
//!   }
//! }
//! ```
//!
//! Known `code` values:
//! - `rate_limited` → session scope (short-term)
//! - `user_weekly_rate_limited` → weekly scope
//! - `user_global_rate_limited:*` → global scope (Pro+ variant)
//!
//! Reset time is encoded as a relative phrase ("in N hours/minutes"), parsed
//! from the message body. If we can't parse it, the scheduler uses the
//! configurable fallback (default 1h) and self-retriggers if still limited.

use crate::usage_limits::{UsageLimit, UsageLimitConfig, UsageLimitScope};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use std::sync::OnceLock;

/// Default patterns for matching against the *error message text*. Used as a
/// fallback when JSON detection isn't available (e.g. text-based scans).
pub const DEFAULT_PATTERNS: &[&str] = &[
    // The general "you've hit a rate limit" / "exceeded your weekly rate limit" lines.
    r"(?i)(?:hit|exceeded|reached) (?:your |the )?(?:weekly |daily )?rate limit",
    r"(?i)you've reached your weekly rate limit",
    r"(?i)you have exceeded your Copilot token usage",
];

/// Known rate-limit error codes, mapped to a scope.
pub const KNOWN_CODES: &[(&str, UsageLimitScope)] = &[
    ("rate_limited", UsageLimitScope::Session),
    ("user_weekly_rate_limited", UsageLimitScope::Weekly),
    // Global variants come in forms like `user_global_rate_limited:pro_plus`.
    ("user_global_rate_limited", UsageLimitScope::Global),
];

static COMPILED: OnceLock<Vec<Regex>> = OnceLock::new();
static DURATION_RE: OnceLock<Regex> = OnceLock::new();

fn compiled_defaults() -> &'static [Regex] {
    COMPILED.get_or_init(|| {
        DEFAULT_PATTERNS
            .iter()
            .map(|src| Regex::new(src).expect("Copilot usage-limit default pattern is valid regex"))
            .collect()
    })
}

fn duration_regex() -> &'static Regex {
    DURATION_RE.get_or_init(|| {
        Regex::new(
            r"(?i)in (\d+)\s*(hour|hours|hr|hrs|minute|minutes|min|mins|second|seconds|sec|secs)",
        )
        .unwrap()
    })
}

fn compile_extras(extras: &[String]) -> Vec<Regex> {
    extras
        .iter()
        .filter_map(|src| match Regex::new(src) {
            Ok(r) => Some(r),
            Err(e) => {
                log::warn!("Ignoring invalid copilot usage-limit pattern {src:?}: {e}");
                None
            }
        })
        .collect()
}

fn parse_relative_duration(msg: &str) -> Option<DateTime<Utc>> {
    let cap = duration_regex().captures(msg)?;
    let n: i64 = cap.get(1)?.as_str().parse().ok()?;
    let unit = cap.get(2)?.as_str().to_lowercase();
    let secs = match unit.as_str() {
        "hour" | "hours" | "hr" | "hrs" => n * 3600,
        "minute" | "minutes" | "min" | "mins" => n * 60,
        "second" | "seconds" | "sec" | "secs" => n,
        _ => return None,
    };
    Some(Utc::now() + Duration::seconds(secs))
}

fn scope_from_code(code: &str) -> UsageLimitScope {
    for (key, scope) in KNOWN_CODES {
        if code == *key || code.starts_with(&format!("{key}:")) {
            return *scope;
        }
    }
    UsageLimitScope::Unknown
}

fn is_known_code(code: &str) -> bool {
    KNOWN_CODES
        .iter()
        .any(|(key, _)| code == *key || code.starts_with(&format!("{key}:")))
}

/// Scan a Copilot events.jsonl event for a usage-limit signal.
///
/// Handles both the canonical envelope:
/// ```json
/// {"type":"error","data":{"error":{"code":"rate_limited","message":"..."}}}
/// ```
/// and a flatter shape some Copilot versions emit:
/// ```json
/// {"type":"error","error":{"code":"rate_limited","message":"..."}}
/// ```
pub fn detect_json(value: &serde_json::Value, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("copilot") {
        return None;
    }

    let event_type = value
        .get("type")
        .or_else(|| value.get("eventType"))
        .and_then(|v| v.as_str());

    // Find the error block via either `data.error` or `error`.
    let error_obj = value
        .get("data")
        .and_then(|v| v.get("error"))
        .or_else(|| value.get("error"))?;

    let code = error_obj.get("code").and_then(|v| v.as_str())?;
    if !is_known_code(code) {
        return None;
    }

    // Only accept on error-shaped events (or when the type is unspecified —
    // some flatter shapes don't carry one).
    if let Some(kind) = event_type {
        if kind != "error" && kind != "model.failed" && !kind.contains("error") {
            return None;
        }
    }

    let message = error_obj
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let reset_at = parse_relative_duration(message);

    Some(UsageLimit {
        provider: "copilot",
        scope: scope_from_code(code),
        reset_at,
        raw: error_obj.to_string(),
    })
}

/// Scan free-form text — for cases where a wrapper saw the message string but
/// not its structured envelope. Falls back to relative-duration parsing.
pub fn detect_text(line: &str, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("copilot") {
        return None;
    }

    let matched = compiled_defaults().iter().any(|re| re.is_match(line))
        || compile_extras(cfg.extra_patterns_for("copilot"))
            .iter()
            .any(|re| re.is_match(line));
    if !matched {
        return None;
    }

    let scope = if line.to_lowercase().contains("weekly") {
        UsageLimitScope::Weekly
    } else if line.to_lowercase().contains("global") {
        UsageLimitScope::Global
    } else {
        UsageLimitScope::Session
    };

    Some(UsageLimit {
        provider: "copilot",
        scope,
        reset_at: parse_relative_duration(line),
        raw: line.to_string(),
    })
}

#[cfg(test)]
#[path = "copilot_usage_limits_tests.rs"]
mod tests;
