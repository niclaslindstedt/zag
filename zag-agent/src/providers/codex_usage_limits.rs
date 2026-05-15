//! Codex usage-limit detection.
//!
//! Single file to update when the Codex CLI changes its limit output.
//!
//! Codex emits this in its `--json` NDJSON stream as the `message` field of an
//! `error` or `turn.failed` event:
//!
//! ```text
//! You've hit your usage limit. To get more access now, send a request to your admin
//! or try again at Mar 20th, 2027 3:36 PM.
//! ```
//!
//! The reset timestamp is a local-timezone human date string — we parse it
//! through several format candidates via chrono and convert to UTC. If parsing
//! fails, `reset_at` is `None` and the scheduler falls back to
//! `default_fallback_secs`.

use crate::usage_limits::{UsageLimit, UsageLimitConfig, UsageLimitScope};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use std::sync::OnceLock;

/// Detection patterns — these answer "is there a usage limit here?"
///
/// Reset-time extraction is decoupled (see `TRY_AGAIN_AT_PATTERN` below) so the
/// detector still fires even when the exact wording around the date drifts.
pub const DEFAULT_PATTERNS: &[&str] = &[
    // Canonical message text.
    r"(?i)you('ve| have)? (?:hit|reached) (?:your |the )?usage limit",
    // Variant some Codex versions emit on weekly/global caps.
    r"(?i)usage limit reset",
];

/// Pattern for extracting the reset-time phrase. Matches "try again at <when>"
/// or "try again after <when>" up to the next sentence boundary.
const TRY_AGAIN_AT_PATTERN: &str =
    r"(?i)try again (?:at|after) (?P<when>[^.\n;]+?)(?:\s*[.;\n]|\s*$)";

/// Date format candidates tried in order against the captured "when" phrase.
///
/// We strip ordinal suffixes (`1st`, `2nd`, `3rd`, `Nth`) and zero-pad
/// single-digit hours before parsing so plain chrono format strings work.
const DATE_FORMATS: &[&str] = &[
    "%b %d, %Y %I:%M %p", // "Mar 20, 2027 03:36 PM"
    "%b %d %Y %I:%M %p",
    "%B %d, %Y %I:%M %p", // "March 20, 2027 03:36 PM"
    "%B %d %Y %I:%M %p",
    "%Y-%m-%d %H:%M",     // "2027-03-20 15:36"
    "%Y-%m-%dT%H:%M:%SZ", // RFC3339-ish
];

static COMPILED: OnceLock<Vec<Regex>> = OnceLock::new();
static TRY_AGAIN_RE: OnceLock<Regex> = OnceLock::new();
static STRIP_ORDINAL: OnceLock<Regex> = OnceLock::new();
static PAD_HOUR_RE: OnceLock<Regex> = OnceLock::new();

fn compiled_defaults() -> &'static [Regex] {
    COMPILED.get_or_init(|| {
        DEFAULT_PATTERNS
            .iter()
            .map(|src| Regex::new(src).expect("Codex usage-limit default pattern is valid regex"))
            .collect()
    })
}

fn strip_ordinal() -> &'static Regex {
    STRIP_ORDINAL.get_or_init(|| Regex::new(r"(\d+)(st|nd|rd|th)").unwrap())
}

fn pad_hour_regex() -> &'static Regex {
    // Matches a single-digit hour before `:MM` so we can zero-pad it.
    PAD_HOUR_RE.get_or_init(|| Regex::new(r"\b(\d):(\d\d)\b").unwrap())
}

fn try_again_re() -> &'static Regex {
    TRY_AGAIN_RE.get_or_init(|| Regex::new(TRY_AGAIN_AT_PATTERN).unwrap())
}

fn parse_reset(when: &str) -> Option<DateTime<Utc>> {
    let cleaned = strip_ordinal().replace_all(when.trim(), "$1").into_owned();
    // Zero-pad single-digit hour: "3:36 PM" → "03:36 PM".
    let padded = pad_hour_regex()
        .replace_all(&cleaned, "0$1:$2")
        .into_owned();
    // Collapse runs of whitespace — `chrono` is strict about this.
    let normalized = padded.split_whitespace().collect::<Vec<_>>().join(" ");

    for fmt in DATE_FORMATS {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, fmt) {
            // Interpret the local-TZ string as the user's local time.
            if let chrono::LocalResult::Single(local) = Local.from_local_datetime(&naive) {
                return Some(local.with_timezone(&Utc));
            }
        }
    }
    None
}

fn extract_reset_from_line(line: &str) -> Option<DateTime<Utc>> {
    try_again_re()
        .captures(line)
        .and_then(|cap| cap.name("when"))
        .and_then(|m| parse_reset(m.as_str()))
}

fn compile_extras(extras: &[String]) -> Vec<Regex> {
    extras
        .iter()
        .filter_map(|src| match Regex::new(src) {
            Ok(r) => Some(r),
            Err(e) => {
                log::warn!("Ignoring invalid codex usage-limit pattern {src:?}: {e}");
                None
            }
        })
        .collect()
}

fn match_against(re: &Regex, line: &str) -> Option<UsageLimit> {
    let cap = re.captures(line)?;
    let raw = cap.get(0)?.as_str().to_string();
    // Reset time is parsed from the full line, not just the matched substring,
    // because the "try again at <date>" phrase often sits after the detection
    // anchor.
    let reset_at = extract_reset_from_line(line);
    Some(UsageLimit {
        provider: "codex",
        scope: UsageLimitScope::Session,
        reset_at,
        raw,
    })
}

/// Scan a single line of text (or message) for a Codex usage-limit signal.
pub fn detect_text(line: &str, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("codex") {
        return None;
    }

    for re in compiled_defaults() {
        if let Some(hit) = match_against(re, line) {
            return Some(hit);
        }
    }

    for re in compile_extras(cfg.extra_patterns_for("codex")) {
        if let Some(hit) = match_against(&re, line) {
            return Some(hit);
        }
    }

    None
}

/// Scan a Codex NDJSON event JSON value for a usage-limit signal.
///
/// Codex `--json` emits `{"type":"error", ...}` / `{"type":"turn.failed", ...}`
/// envelopes carrying the limit message in a `message` or `error` field.
pub fn detect_json(value: &serde_json::Value, cfg: &UsageLimitConfig) -> Option<UsageLimit> {
    if !cfg.enabled_for("codex") {
        return None;
    }
    let kind = value.get("type").and_then(|v| v.as_str())?;
    if !matches!(kind, "error" | "turn.failed" | "turn_failed") {
        return None;
    }

    // Collect candidate text fields.
    let mut candidates: Vec<&str> = Vec::new();
    for k in ["message", "error", "detail", "reason"] {
        if let Some(s) = value.get(k).and_then(|v| v.as_str()) {
            candidates.push(s);
        }
    }
    // Also try nested `error.message`.
    if let Some(s) = value
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
    {
        candidates.push(s);
    }

    for cand in candidates {
        if let Some(hit) = detect_text(cand, cfg) {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
#[path = "codex_usage_limits_tests.rs"]
mod tests;
