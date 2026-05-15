use super::*;
use crate::usage_limits::UsageLimitConfig;

#[test]
fn detects_rate_limited_session_scope() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "data": {
            "error": {
                "message": "Sorry, you've hit a rate limit. Please try again in 2 hours.",
                "code": "rate_limited"
            }
        }
    });
    let hit = detect_json(&value, &cfg).expect("should detect");
    assert_eq!(hit.provider, "copilot");
    assert_eq!(hit.scope, UsageLimitScope::Session);
    let reset = hit.reset_at.expect("reset parses from relative phrase");
    let secs = (reset - Utc::now()).num_seconds();
    assert!((7100..=7300).contains(&secs), "expected ~2h, got {secs}s");
}

#[test]
fn detects_weekly_scope_from_code() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "error": {
            "code": "user_weekly_rate_limited",
            "message": "You've exceeded your weekly rate limit. Please wait 12 hours.",
        }
    });
    let hit = detect_json(&value, &cfg).expect("should detect");
    assert_eq!(hit.scope, UsageLimitScope::Weekly);
    // Note: "Please wait 12 hours" doesn't match the "in N hours" pattern,
    // so reset_at is None — fallback applies.
    assert!(hit.reset_at.is_none());
}

#[test]
fn detects_global_scope_with_subcode() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "error": {
            "code": "user_global_rate_limited:pro_plus",
            "message": "Sorry, you have been rate-limited."
        }
    });
    let hit = detect_json(&value, &cfg).expect("should detect");
    assert_eq!(hit.scope, UsageLimitScope::Global);
}

#[test]
fn parses_in_n_minutes_phrase() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "error": {
            "code": "rate_limited",
            "message": "Please try again in 30 minutes."
        }
    });
    let hit = detect_json(&value, &cfg).expect("should detect");
    let secs = (hit.reset_at.unwrap() - Utc::now()).num_seconds();
    assert!((1700..=1900).contains(&secs), "got {secs}");
}

#[test]
fn ignores_unrelated_error_codes() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "error": {
            "code": "unauthorized",
            "message": "Bad token"
        }
    });
    assert!(detect_json(&value, &cfg).is_none());
}

#[test]
fn ignores_non_error_events_with_rate_limited_code() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "tool.execution_complete",
        "error": {
            "code": "rate_limited",
            "message": "in 1 hour"
        }
    });
    // Not an error event — don't match.
    assert!(detect_json(&value, &cfg).is_none());
}

#[test]
fn detect_text_falls_back_for_loose_capture() {
    let cfg = UsageLimitConfig::default();
    let line = "Server Error: Sorry, you've exceeded your weekly rate limit.";
    let hit = detect_text(line, &cfg).expect("text-only fallback detection");
    assert_eq!(hit.provider, "copilot");
    assert_eq!(hit.scope, UsageLimitScope::Weekly);
}

#[test]
fn does_not_match_unrelated_text() {
    let cfg = UsageLimitConfig::default();
    assert!(detect_text("Hello world", &cfg).is_none());
    assert!(detect_text("Copilot response", &cfg).is_none());
}
