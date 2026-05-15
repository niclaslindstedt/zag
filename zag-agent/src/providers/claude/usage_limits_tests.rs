use super::*;
use crate::usage_limits::{UsageLimitConfig, UsageLimitProviderOverride};

#[test]
fn detects_bare_usage_limit_with_epoch() {
    let cfg = UsageLimitConfig::default();
    let line = "Claude AI usage limit reached|1760000400";
    let hit = detect_text(line, &cfg).expect("should detect");
    assert_eq!(hit.provider, "claude");
    assert_eq!(hit.scope, UsageLimitScope::Session);
    assert_eq!(hit.reset_at.unwrap().timestamp(), 1_760_000_400);
    assert_eq!(hit.raw, "Claude AI usage limit reached|1760000400");
}

#[test]
fn detects_weekly_scope() {
    let cfg = UsageLimitConfig::default();
    let line = "Claude AI weekly usage limit reached|1760000400";
    let hit = detect_text(line, &cfg).expect("should detect weekly");
    assert_eq!(hit.scope, UsageLimitScope::Weekly);
    assert_eq!(hit.reset_at.unwrap().timestamp(), 1_760_000_400);
}

#[test]
fn detects_global_scope() {
    let cfg = UsageLimitConfig::default();
    let line = "Claude AI global usage limit reached|1760000400";
    let hit = detect_text(line, &cfg).expect("should detect global");
    assert_eq!(hit.scope, UsageLimitScope::Global);
}

#[test]
fn detects_when_embedded_in_larger_text() {
    let cfg = UsageLimitConfig::default();
    let line = "I'm sorry, but Claude AI usage limit reached|1760000400 — try again later.";
    assert!(detect_text(line, &cfg).is_some());
}

#[test]
fn does_not_match_unrelated_text() {
    let cfg = UsageLimitConfig::default();
    assert!(detect_text("Hello world", &cfg).is_none());
    assert!(detect_text("Claude AI is great", &cfg).is_none());
    assert!(detect_text("usage limit reached", &cfg).is_none());
}

#[test]
fn respects_disabled_flag() {
    let cfg = UsageLimitConfig {
        enabled: false,
        ..Default::default()
    };
    let line = "Claude AI usage limit reached|1760000400";
    assert!(detect_text(line, &cfg).is_none());
}

#[test]
fn respects_per_provider_disabled() {
    let mut cfg = UsageLimitConfig::default();
    cfg.providers.insert(
        "claude".to_string(),
        UsageLimitProviderOverride {
            enabled: Some(false),
            ..Default::default()
        },
    );
    let line = "Claude AI usage limit reached|1760000400";
    assert!(detect_text(line, &cfg).is_none());
}

#[test]
fn extra_patterns_match_when_defaults_dont() {
    let mut cfg = UsageLimitConfig::default();
    cfg.providers.insert(
        "claude".to_string(),
        UsageLimitProviderOverride {
            extra_patterns: vec![r"hypothetical-new-limit-format=(\d+)".to_string()],
            ..Default::default()
        },
    );
    let line = "Server: hypothetical-new-limit-format=1760000400 sorry";
    let hit = detect_text(line, &cfg).expect("user pattern should match");
    assert_eq!(hit.provider, "claude");
}

#[test]
fn detects_api_retry_envelope() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "system",
        "subtype": "api_retry",
        "attempt": 1,
        "max_retries": 5,
        "retry_delay_ms": 2000,
        "error_status": 429,
        "error": "rate_limit",
    });
    let hit = detect_json(&value, &cfg).expect("should detect api_retry rate limit");
    assert_eq!(hit.provider, "claude");
    assert!(hit.reset_at.is_none()); // structured envelope, no reset time
}

#[test]
fn detects_result_error_envelope() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "result",
        "subtype": "error",
        "is_error": true,
        "error": "rate_limit",
    });
    assert!(detect_json(&value, &cfg).is_some());
}

#[test]
fn ignores_non_rate_limit_system_events() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "system",
        "subtype": "init",
        "session_id": "abc",
    });
    assert!(detect_json(&value, &cfg).is_none());

    let other = serde_json::json!({
        "type": "system",
        "subtype": "api_retry",
        "error": "server_error",
    });
    assert!(detect_json(&other, &cfg).is_none());
}
