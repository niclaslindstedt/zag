//! CLI-side sanity checks. The bulk of the envelope-wrapping / session-resolution
//! coverage lives in `zag-orch/src/messaging_tests.rs` now that the logic was
//! lifted into the library; these tests just confirm the re-exported shim is
//! still reachable from the CLI module path.

use super::*;
use std::sync::Mutex;
use zag_orch::messaging::{SenderInfo, wrap_agent_message};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_wrap_agent_message_full_info() {
    let sender = SenderInfo {
        session_id: "abc-123".to_string(),
        name: None,
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
fn test_maybe_wrap_message_raw_skips_wrapping() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let msg = "plain message";
    let result = maybe_wrap_message(msg, true);
    assert_eq!(result, msg);
}

#[test]
fn test_maybe_wrap_message_no_session_returns_raw() {
    let _lock = ENV_MUTEX.lock().unwrap();

    let original = std::env::var("ZAG_SESSION_ID").ok();
    unsafe { std::env::remove_var("ZAG_SESSION_ID") };

    let msg = "plain message";
    let result = maybe_wrap_message(msg, false);
    assert_eq!(result, msg);

    if let Some(val) = original {
        unsafe { std::env::set_var("ZAG_SESSION_ID", val) };
    }
}
