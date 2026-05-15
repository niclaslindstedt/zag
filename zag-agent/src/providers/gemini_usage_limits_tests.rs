use super::*;
use crate::usage_limits::UsageLimitConfig;

const REAL_429: &str = r#"✕ [API Error: [{
  "error": {
    "code": 429,
    "message": "Quota exceeded for quota metric 'Gemini 2.5 Pro Requests' and limit 'Gemini 2.5 Pro Requests per day per user per tier' of service 'cloudcode-pa.googleapis.com' for consumer 'project_number:681255809395'.",
    "status": "RESOURCE_EXHAUSTED",
    "details": [
      {
        "@type": "type.googleapis.com/google.rpc.ErrorInfo",
        "reason": "RATE_LIMIT_EXCEEDED",
        "domain": "googleapis.com",
        "metadata": {
          "quota_metric": "cloudcode-pa.googleapis.com/gemini_2_5_pro_requests",
          "quota_limit": "Gemini25ProRequestsPerDay"
        }
      }
    ]
  }
}]]"#;

#[test]
fn detects_real_429_via_text() {
    let cfg = UsageLimitConfig::default();
    let hit = detect_text(REAL_429, &cfg).expect("should detect");
    assert_eq!(hit.provider, "gemini");
    assert_eq!(hit.scope, UsageLimitScope::Daily);
    // No retryDelay → no reset_at, fallback applies.
    assert!(hit.reset_at.is_none());
}

#[test]
fn detects_retry_delay_when_present() {
    let cfg = UsageLimitConfig::default();
    let blob = r#"✕ [API Error: { "error": { "code": 429, "status": "RESOURCE_EXHAUSTED", "details": [{ "retryDelay": "45s" }] } }]"#;
    let hit = detect_text(blob, &cfg).expect("should detect");
    let secs = (hit.reset_at.unwrap() - Utc::now()).num_seconds();
    assert!((40..=60).contains(&secs), "got {secs}");
}

#[test]
fn detects_free_tier_message_with_no_envelope() {
    let cfg = UsageLimitConfig::default();
    let line = "Rate Limit Exceeded. You've hit the API rate limit. This is likely due to free tier limits.";
    let hit = detect_text(line, &cfg).expect("should detect free-tier message");
    assert_eq!(hit.provider, "gemini");
}

#[test]
fn detects_json_value_with_resource_exhausted() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "error": {
            "code": 429,
            "status": "RESOURCE_EXHAUSTED",
            "message": "Quota exceeded",
            "details": [{ "reason": "RATE_LIMIT_EXCEEDED" }]
        }
    });
    assert!(detect_json(&value, &cfg).is_some());
}

#[test]
fn detects_json_array_form() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!([{
        "error": {
            "code": 429,
            "status": "RESOURCE_EXHAUSTED"
        }
    }]);
    assert!(detect_json(&value, &cfg).is_some());
}

#[test]
fn classifies_per_week_quota_as_weekly() {
    let cfg = UsageLimitConfig::default();
    let blob = r#"[API Error: { "error": { "code": 429, "status": "RESOURCE_EXHAUSTED", "details": [{ "metadata": { "quota_limit": "Gemini25ProRequestsPerWeek" } }] } }]"#;
    let hit = detect_text(blob, &cfg).expect("should detect");
    assert_eq!(hit.scope, UsageLimitScope::Weekly);
}

#[test]
fn does_not_match_unrelated_text() {
    let cfg = UsageLimitConfig::default();
    assert!(detect_text("Hello world", &cfg).is_none());
    assert!(detect_text("API Error: 500 Internal Server Error", &cfg).is_none());
}

#[test]
fn ignores_non_429_json() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "error": { "code": 500, "status": "INTERNAL" }
    });
    assert!(detect_json(&value, &cfg).is_none());
}
