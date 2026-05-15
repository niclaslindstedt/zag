use super::*;
use chrono::Duration;

fn make_hit(provider: &'static str, reset: Option<DateTime<Utc>>) -> UsageLimit {
    UsageLimit {
        provider,
        scope: UsageLimitScope::Session,
        reset_at: reset,
        raw: "test".to_string(),
    }
}

#[test]
fn compute_resume_uses_provider_reset_time_with_jitter() {
    let cfg = UsageLimitConfig::default();
    let target = Utc::now() + Duration::seconds(120);
    let hit = make_hit("claude", Some(target));

    let (scheduled, fallback_used) = compute_resume_at(&hit, &cfg);
    assert!(!fallback_used);
    // Jitter is 30s by default, so scheduled = target + 30s ± rounding.
    let diff = (scheduled - target).num_seconds();
    assert!(
        (30..=31).contains(&diff),
        "expected ~30s jitter, got {diff}"
    );
}

#[test]
fn compute_resume_falls_back_when_reset_is_none() {
    let cfg = UsageLimitConfig::default();
    let hit = make_hit("codex", None);
    let before = Utc::now();
    let (scheduled, fallback_used) = compute_resume_at(&hit, &cfg);
    assert!(fallback_used);
    // Default fallback is 3600s + 30s jitter = ~3630s out.
    let secs = (scheduled - before).num_seconds();
    assert!(
        (3600..=3700).contains(&secs),
        "expected ~3630s fallback, got {secs}"
    );
}

#[test]
fn compute_resume_respects_per_provider_fallback_override() {
    let mut cfg = UsageLimitConfig::default();
    cfg.providers.insert(
        "copilot".to_string(),
        UsageLimitProviderOverride {
            fallback_secs: Some(60),
            ..Default::default()
        },
    );
    let hit = make_hit("copilot", None);
    let before = Utc::now();
    let (scheduled, _) = compute_resume_at(&hit, &cfg);
    let secs = (scheduled - before).num_seconds();
    assert!((60..=100).contains(&secs), "got {secs}");
}

#[test]
fn compute_resume_caps_at_max_wait_secs() {
    let cfg = UsageLimitConfig {
        max_wait_secs: 10,
        ..Default::default()
    };
    // Reset is 1 year out — should be capped to now + 10s.
    let target = Utc::now() + Duration::days(365);
    let hit = make_hit("claude", Some(target));
    let before = Utc::now();
    let (scheduled, _) = compute_resume_at(&hit, &cfg);
    let secs = (scheduled - before).num_seconds();
    assert!(secs <= 12, "expected ≤10s cap (got {secs})");
}

#[test]
fn compute_resume_clamps_past_reset_to_near_now() {
    let cfg = UsageLimitConfig::default();
    let target = Utc::now() - Duration::seconds(60);
    let hit = make_hit("claude", Some(target));
    let before = Utc::now();
    let (scheduled, _) = compute_resume_at(&hit, &cfg);
    let secs = (scheduled - before).num_seconds();
    // Past reset is replaced by now + jitter (~30s), then capped.
    assert!((25..=35).contains(&secs), "got {secs}");
}

#[test]
fn enabled_for_respects_global_and_per_provider() {
    let mut cfg = UsageLimitConfig::default();
    assert!(cfg.enabled_for("claude"));

    cfg.enabled = false;
    assert!(!cfg.enabled_for("claude"));

    cfg.enabled = true;
    cfg.providers.insert(
        "codex".to_string(),
        UsageLimitProviderOverride {
            enabled: Some(false),
            ..Default::default()
        },
    );
    assert!(!cfg.enabled_for("codex"));
    assert!(cfg.enabled_for("claude"));
}

#[test]
fn resume_message_per_provider_override() {
    let mut cfg = UsageLimitConfig::default();
    assert_eq!(cfg.resume_message_for("claude"), "Continue");
    cfg.providers.insert(
        "copilot".to_string(),
        UsageLimitProviderOverride {
            resume_message: Some("Please continue.".to_string()),
            ..Default::default()
        },
    );
    assert_eq!(cfg.resume_message_for("copilot"), "Please continue.");
    assert_eq!(cfg.resume_message_for("claude"), "Continue");
}
