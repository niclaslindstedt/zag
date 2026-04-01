use super::*;

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
