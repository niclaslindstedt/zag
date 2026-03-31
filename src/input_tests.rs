use super::*;

#[test]
fn test_wrap_agent_message_full_info() {
    let sender = SenderInfo {
        session_id: "abc-123".to_string(),
        provider: Some("claude".to_string()),
        model: Some("opus".to_string()),
    };
    let result = wrap_agent_message("hello world", &sender);
    assert!(result.contains("<agent-message>"));
    assert!(result.contains("</agent-message>"));
    assert!(result.contains(r#"<from session="abc-123" provider="claude" model="opus"/>"#));
    assert!(
        result
            .contains(r#"<reply-with>zag input --session abc-123 "your reply here"</reply-with>"#)
    );
    assert!(result.contains("<body>\nhello world\n</body>"));
}

#[test]
fn test_wrap_agent_message_missing_provider_and_model() {
    let sender = SenderInfo {
        session_id: "xyz-789".to_string(),
        provider: None,
        model: None,
    };
    let result = wrap_agent_message("test msg", &sender);
    assert!(result.contains(r#"provider="unknown""#));
    assert!(result.contains(r#"model="unknown""#));
    assert!(result.contains("test msg"));
}

#[test]
fn test_maybe_wrap_message_raw_skips_wrapping() {
    // Even if env vars were set, raw=true should return the message as-is
    let msg = "plain message";
    let result = maybe_wrap_message(msg, true);
    assert_eq!(result, msg);
}

#[test]
fn test_maybe_wrap_message_no_session_returns_raw() {
    // Temporarily ensure ZAG_SESSION_ID is not set
    let original = std::env::var("ZAG_SESSION_ID").ok();
    unsafe { std::env::remove_var("ZAG_SESSION_ID") };

    let msg = "plain message";
    let result = maybe_wrap_message(msg, false);
    assert_eq!(result, msg);

    // Restore
    if let Some(val) = original {
        unsafe { std::env::set_var("ZAG_SESSION_ID", val) };
    }
}

#[test]
fn test_sender_info_from_env_returns_none_without_session() {
    let original = std::env::var("ZAG_SESSION_ID").ok();
    unsafe { std::env::remove_var("ZAG_SESSION_ID") };

    assert!(SenderInfo::from_env().is_none());

    if let Some(val) = original {
        unsafe { std::env::set_var("ZAG_SESSION_ID", val) };
    }
}

#[test]
fn test_sender_info_from_env_reads_vars() {
    let orig_sid = std::env::var("ZAG_SESSION_ID").ok();
    let orig_prov = std::env::var("ZAG_PROVIDER").ok();
    let orig_model = std::env::var("ZAG_MODEL").ok();

    unsafe {
        std::env::set_var("ZAG_SESSION_ID", "test-session-42");
        std::env::set_var("ZAG_PROVIDER", "gemini");
        std::env::set_var("ZAG_MODEL", "flash");
    }

    let info = SenderInfo::from_env().expect("should detect session");
    assert_eq!(info.session_id, "test-session-42");
    assert_eq!(info.provider.as_deref(), Some("gemini"));
    assert_eq!(info.model.as_deref(), Some("flash"));

    // Restore
    unsafe {
        match orig_sid {
            Some(v) => std::env::set_var("ZAG_SESSION_ID", v),
            None => std::env::remove_var("ZAG_SESSION_ID"),
        }
        match orig_prov {
            Some(v) => std::env::set_var("ZAG_PROVIDER", v),
            None => std::env::remove_var("ZAG_PROVIDER"),
        }
        match orig_model {
            Some(v) => std::env::set_var("ZAG_MODEL", v),
            None => std::env::remove_var("ZAG_MODEL"),
        }
    }
}
