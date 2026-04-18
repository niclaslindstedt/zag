use super::*;

fn clear_sender_env() {
    unsafe {
        std::env::remove_var("ZAG_SESSION_ID");
        std::env::remove_var("ZAG_SESSION_NAME");
        std::env::remove_var("ZAG_PROVIDER");
        std::env::remove_var("ZAG_MODEL");
    }
}

#[test]
fn sender_info_none_outside_session() {
    clear_sender_env();
    assert!(SenderInfo::from_env().is_none());
}

#[test]
fn sender_info_populated_from_env() {
    unsafe {
        std::env::set_var("ZAG_SESSION_ID", "sess-1");
        std::env::set_var("ZAG_SESSION_NAME", "alpha");
        std::env::set_var("ZAG_PROVIDER", "claude");
        std::env::set_var("ZAG_MODEL", "sonnet");
    }
    let info = SenderInfo::from_env().expect("env was set");
    assert_eq!(info.session_id, "sess-1");
    assert_eq!(info.name.as_deref(), Some("alpha"));
    assert_eq!(info.provider.as_deref(), Some("claude"));
    assert_eq!(info.model.as_deref(), Some("sonnet"));
    clear_sender_env();
}

#[test]
fn wrap_agent_message_includes_reply_target_with_name() {
    let sender = SenderInfo {
        session_id: "sess-1".to_string(),
        name: Some("alpha".to_string()),
        provider: Some("claude".to_string()),
        model: Some("sonnet".to_string()),
    };
    let wrapped = wrap_agent_message("hi there", &sender);
    assert!(wrapped.contains("<agent-message>"));
    assert!(wrapped.contains("name=\"alpha\""));
    assert!(wrapped.contains("provider=\"claude\""));
    assert!(wrapped.contains("model=\"sonnet\""));
    assert!(wrapped.contains("zag input --name alpha"));
    assert!(wrapped.contains("hi there"));
}

#[test]
fn wrap_agent_message_uses_session_id_when_name_missing() {
    let sender = SenderInfo {
        session_id: "sess-1".to_string(),
        name: None,
        provider: Some("codex".to_string()),
        model: None,
    };
    let wrapped = wrap_agent_message("hi", &sender);
    assert!(wrapped.contains("zag input --session sess-1"));
    assert!(!wrapped.contains("name="));
}

#[test]
fn maybe_wrap_message_passthrough_when_raw() {
    clear_sender_env();
    assert_eq!(maybe_wrap_message("payload", true), "payload");
}

#[test]
fn maybe_wrap_message_passthrough_without_session() {
    clear_sender_env();
    assert_eq!(maybe_wrap_message("payload", false), "payload");
}

#[test]
fn maybe_wrap_message_wraps_when_inside_session() {
    unsafe {
        std::env::set_var("ZAG_SESSION_ID", "sess-42");
        std::env::set_var("ZAG_PROVIDER", "claude");
    }
    let wrapped = maybe_wrap_message("payload", false);
    assert!(wrapped.contains("<agent-message>"));
    assert!(wrapped.contains("sess-42"));
    assert!(wrapped.contains("payload"));
    clear_sender_env();
}

#[test]
fn broadcast_result_counts() {
    let result = BroadcastResult {
        outcomes: vec![
            BroadcastOutcome {
                session_id: "a".to_string(),
                result: Ok(()),
            },
            BroadcastOutcome {
                session_id: "b".to_string(),
                result: Err("nope".to_string()),
            },
        ],
    };
    assert_eq!(result.sent(), 1);
    assert_eq!(result.failed(), 1);
    assert_eq!(result.total(), 2);
}
