use super::*;

#[test]
fn test_resolve_process_id_literal() {
    let result = resolve_process_id("abc-123").unwrap();
    assert_eq!(result, "abc-123");
}

#[test]
fn test_resolve_process_id_not_self() {
    // Any string that is not "self" should be returned as-is,
    // regardless of environment state.
    assert_eq!(resolve_process_id("some-uuid").unwrap(), "some-uuid");
    assert_eq!(resolve_process_id("").unwrap(), "");
    assert_eq!(resolve_process_id("Self").unwrap(), "Self");
}

/// Env-var-dependent tests are grouped in one test to avoid races with
/// parallel tests that may also mutate the environment.
#[test]
fn test_resolve_process_id_self_env_variants() {
    // Without the env var set, "self" should error.
    unsafe {
        std::env::remove_var("ZAG_PROCESS_ID");
    }
    let err = resolve_process_id("self").unwrap_err();
    assert!(err.to_string().contains("ZAG_PROCESS_ID is not set"));

    // With the env var set, "self" should resolve to its value.
    unsafe {
        std::env::set_var("ZAG_PROCESS_ID", "proc-self-test");
    }
    let result = resolve_process_id("self").unwrap();
    assert_eq!(result, "proc-self-test");

    // Clean up
    unsafe {
        std::env::remove_var("ZAG_PROCESS_ID");
    }
}

fn make_entry(status: &str) -> ProcessEntry {
    ProcessEntry {
        id: "test-id".to_string(),
        pid: 99999,
        session_id: None,
        provider: "claude".to_string(),
        model: "sonnet".to_string(),
        command: "exec".to_string(),
        prompt: None,
        started_at: "2026-01-01T00:00:00Z".to_string(),
        status: status.to_string(),
        exit_code: None,
        exited_at: None,
        root: None,
        parent_process_id: None,
        parent_session_id: None,
    }
}

#[test]
fn test_resolve_live_status_exited() {
    let entry = make_entry("exited");
    assert_eq!(resolve_live_status(&entry), "exited");
}

#[test]
fn test_resolve_live_status_killed() {
    let entry = make_entry("killed");
    assert_eq!(resolve_live_status(&entry), "killed");
}

#[test]
fn test_resolve_live_status_unknown_status() {
    let entry = make_entry("something_else");
    assert_eq!(resolve_live_status(&entry), "unknown");
}

#[test]
fn test_resolve_live_status_running_dead_process() {
    // Use a PID that almost certainly doesn't exist
    let entry = make_entry("running");
    // PID 99999 is unlikely to be a real process in test
    let status = resolve_live_status(&entry);
    // Should be either "running" (if PID happens to exist) or "dead"
    assert!(status == "running" || status == "dead");
}
