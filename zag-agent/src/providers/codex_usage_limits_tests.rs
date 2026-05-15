use super::*;
use crate::usage_limits::UsageLimitConfig;

#[test]
fn detects_canonical_message_with_ordinal_date() {
    let cfg = UsageLimitConfig::default();
    let line = "You've hit your usage limit. To get more access now, send a request to your admin or try again at Mar 20th, 2027 3:36 PM.";
    let hit = detect_text(line, &cfg).expect("should detect");
    assert_eq!(hit.provider, "codex");
    assert_eq!(hit.scope, UsageLimitScope::Session);
    let reset = hit.reset_at.expect("reset parses");
    // 2027-03-20 15:36 in the test's local TZ — verify the components rather
    // than the absolute UTC instant.
    let local = reset.with_timezone(&Local);
    assert_eq!(
        local.format("%Y-%m-%d %H:%M").to_string(),
        "2027-03-20 15:36"
    );
}

#[test]
fn detects_without_ordinal_suffix() {
    let cfg = UsageLimitConfig::default();
    let line = "You've hit your usage limit. ... try again at Mar 20, 2027 3:36 PM.";
    let hit = detect_text(line, &cfg).expect("should detect");
    assert!(hit.reset_at.is_some());
}

#[test]
fn detects_full_month_name() {
    let cfg = UsageLimitConfig::default();
    let line = "You've hit your usage limit. ... try again at March 20th, 2027 3:36 PM.";
    let hit = detect_text(line, &cfg).expect("should detect");
    assert!(hit.reset_at.is_some());
}

#[test]
fn detects_iso_form() {
    let cfg = UsageLimitConfig::default();
    let line = "You've hit your usage limit; try again at 2027-03-20 15:36.";
    let hit = detect_text(line, &cfg).expect("should detect");
    assert!(hit.reset_at.is_some());
}

#[test]
fn returns_none_reset_when_date_unparseable() {
    let cfg = UsageLimitConfig::default();
    let line = "You've hit your usage limit. try again at sometime soon.";
    let hit = detect_text(line, &cfg).expect("should still detect (no reset)");
    assert!(hit.reset_at.is_none());
    assert!(hit.raw.contains("You've hit your usage limit"));
}

#[test]
fn does_not_match_unrelated_text() {
    let cfg = UsageLimitConfig::default();
    assert!(detect_text("Hello world", &cfg).is_none());
    assert!(detect_text("Codex completed successfully", &cfg).is_none());
}

#[test]
fn detects_in_error_event_json() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "message": "You've hit your usage limit. try again at Mar 20th, 2027 3:36 PM."
    });
    let hit = detect_json(&value, &cfg).expect("should detect from error event");
    assert!(hit.reset_at.is_some());
}

#[test]
fn detects_in_turn_failed_event_json() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "turn.failed",
        "error": "You've hit your usage limit; try again at 2027-03-20 15:36."
    });
    assert!(detect_json(&value, &cfg).is_some());
}

#[test]
fn ignores_unrelated_error_event() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "message": "Connection refused",
    });
    assert!(detect_json(&value, &cfg).is_none());
}

#[test]
fn detects_nested_error_message_field() {
    let cfg = UsageLimitConfig::default();
    let value = serde_json::json!({
        "type": "error",
        "error": {
            "message": "You've hit your usage limit. try again at Mar 20, 2027 3:36 PM."
        }
    });
    assert!(detect_json(&value, &cfg).is_some());
}
