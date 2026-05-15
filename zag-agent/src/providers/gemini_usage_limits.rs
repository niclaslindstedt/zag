//! Gemini usage-limit detection.
//!
//! Single file to update when the Gemini CLI changes its limit output.
//!
//! Gemini prints rate-limit errors to stderr prefixed with `✕ [API Error:`,
//! wrapping the raw Google API 429 response verbatim:
//!
//! ```text
//! ✕ [API Error: [{
//!   "error": {
//!     "code": 429,
//!     "message": "Quota exceeded for quota metric 'Gemini 2.5 Pro Requests' ...",
//!     "status": "RESOURCE_EXHAUSTED",
//!     "details": [{
//!       "@type": "type.googleapis.com/google.rpc.ErrorInfo",
//!       "reason": "RATE_LIMIT_EXCEEDED",
//!       "domain": "googleapis.com",
//!       "metadata": { "quota_limit": "Gemini25ProRequestsPerDay" }
//!     }]
//!   }
//! }]]
//! ```
//!
//! There's no reliably-surfaced reset timestamp. We look for `retryDelay`
//! (e.g. `"30s"`) when present, but in the common Daily-quota case there is
//! none and the scheduler falls back to `default_fallback_secs`.

use crate::usage_limits::{UsageLimit, UsageLimitConfig, UsageLimitScope};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use std::sync::OnceLock;

/// Default text patterns. When matching against stderr, the line that includes
/// the JSON envelope is sometimes split across multiple physical lines —
/// callers should join multi-line buffers before passing to `detect_text`.
pub const DEFAULT_PATTERNS: &[&str] = &[
    r"\[API Error:.*?\bcode\s*:\s*429",
    r"RESOURCE_EXHAUSTED",
    r"RATE_LIMIT_EXCEEDED",
    r"(?i)you've hit the API rate limit",
];

static COMPILED: OnceLock<Vec<Regex>> = OnceLock::new();
static RETRY_DELAY_RE: OnceLock<Regex> = OnceLock::new();
static QUOTA_NAME_RE: OnceLock<Regex> = OnceLock::new();

fn compiled_defaults() -> &'static [Regex] {
    COMPILED.get_or_init(|| {
        DEFAULT_PATTERNS
            .iter()
            .map(|src| Regex::new(src).expect("Gemini usage-limit default pattern is valid regex"))
            .collect()
    })
}

fn retry_delay_regex() -> &'static Regex {
    // Matches `"retryDelay": "30s"` or `"retryDelay":"30.5s"`.
    RETRY_DELAY_RE.get_or_init(|| Regex::new(r#""retryDelay"\s*:\s*"(\d+(?:\.\d+)?)s""#).unwrap())
}

fn quota_name_regex() -> &'static Regex {
    QUOTA_NAME_RE.get_or_init(|| Regex::new(r#""quota_limit"\s*:\s*"([^"]+)""#).unwrap())
}

fn compile_extras(extras: &[String]) -> Vec<Regex> {
    extras
        .iter()
        .filter_map(|src| match Regex::new(src) {
            Ok(r) => Some(r),
            Err(e) => {
                log::warn!("Ignoring invalid gemini usage-limit pattern {src:?}: {e}");
                None
            }
        })
        .collect()
}

fn parse_retry_delay(blob: &str) -> Option<DateTime<Utc>> {
    let cap = retry_delay_regex().captures(blob)?;
    let secs: f64 = cap.get(1)?.as_str().parse().ok()?;
    Some(Utc::now() + Duration::milliseconds((secs * 1000.0) as i64))
}

fn scope_from_blob(blob: &str) -> UsageLimitScope {
    let lower = blob.to_lowercase();
    if let Some(cap) = quota_name_regex().captures(blob) {
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if name.to_lowercase().contains("perday") {
            return UsageLimitScope::Daily;
        }
        if name.to_lowercase().contains("perweek") {
            return UsageLimitScope::Weekly;
        }
    }
    if lower.contains("per day") || lower.contains("perday") {
        UsageLimitScope::Daily
    } else if lower.contains("per week") || lower.contains("perweek") {
        UsageLimitScope::Weekly
    } else {
        UsageLimitScope::Session
    }
}

fn match_any(line: &str, patterns: &[Regex]) -> Option<usize> {
    for (i, re) in patterns.iter().enumerate() {
        if re.is_match(line) {
            return Some(i);
        }
    }
    None
}

/// Scan a (multi-line, joined) stderr blob for a Gemini usage-limit signal.
pub fn detect_text(line: &str, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("gemini") {
        return None;
    }

    let matched = match_any(line, compiled_defaults()).is_some()
        || match_any(line, &compile_extras(cfg.extra_patterns_for("gemini"))).is_some();
    if !matched {
        return None;
    }

    Some(UsageLimit {
        provider: "gemini",
        scope: scope_from_blob(line),
        reset_at: parse_retry_delay(line),
        raw: line.to_string(),
    })
}

/// Scan a Gemini JSON error blob (the embedded array under `✕ [API Error:`).
pub fn detect_json(value: &serde_json::Value, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("gemini") {
        return None;
    }

    // Top-level may be an object or a single-element array.
    let inner = match value {
        serde_json::Value::Array(arr) => arr.first()?,
        other => other,
    };

    let error = inner.get("error")?;
    let code = error.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    let status = error.get("status").and_then(|v| v.as_str()).unwrap_or("");

    if code != 429 && status != "RESOURCE_EXHAUSTED" {
        return None;
    }

    let raw = error.to_string();
    let scope = scope_from_blob(&raw);
    Some(UsageLimit {
        provider: "gemini",
        scope,
        reset_at: parse_retry_delay(&raw),
        raw,
    })
}

#[cfg(test)]
#[path = "gemini_usage_limits_tests.rs"]
mod tests;
