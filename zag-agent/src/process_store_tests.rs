use super::*;
use tempfile::TempDir;

fn make_entry(id: &str, pid: u32, status: &str) -> ProcessEntry {
    ProcessEntry {
        id: id.to_string(),
        pid,
        session_id: None,
        provider: "claude".to_string(),
        model: "sonnet".to_string(),
        command: "exec".to_string(),
        prompt: Some("hello".to_string()),
        started_at: "2026-03-28T10:00:00Z".to_string(),
        status: status.to_string(),
        exit_code: None,
        exited_at: None,
        root: None,
        parent_process_id: None,
        parent_session_id: None,
    }
}

#[test]
fn add_and_find() {
    let mut store = ProcessStore::default();
    store.add(make_entry("id-1", 100, "running"));
    assert!(store.find("id-1").is_some());
    assert!(store.find("id-2").is_none());
}

#[test]
fn add_replaces_existing() {
    let mut store = ProcessStore::default();
    store.add(make_entry("id-1", 100, "running"));
    store.add(make_entry("id-1", 200, "exited"));
    assert_eq!(store.processes.len(), 1);
    assert_eq!(store.find("id-1").unwrap().pid, 200);
}

#[test]
fn update_status() {
    let mut store = ProcessStore::default();
    store.add(make_entry("id-1", 100, "running"));
    store.update_status("id-1", "exited", Some(0));
    let e = store.find("id-1").unwrap();
    assert_eq!(e.status, "exited");
    assert_eq!(e.exit_code, Some(0));
    assert!(e.exited_at.is_some());
}

#[test]
fn list_recent_sorted() {
    let mut store = ProcessStore::default();
    let mut e1 = make_entry("id-1", 1, "exited");
    e1.started_at = "2026-03-28T09:00:00Z".to_string();
    let mut e2 = make_entry("id-2", 2, "running");
    e2.started_at = "2026-03-28T10:00:00Z".to_string();
    store.add(e1);
    store.add(e2);
    let list = store.list_recent(None);
    assert_eq!(list[0].id, "id-2"); // newest first
    assert_eq!(list[1].id, "id-1");
}

#[test]
fn list_recent_limit() {
    let mut store = ProcessStore::default();
    for i in 0..5 {
        store.add(make_entry(&format!("id-{}", i), i as u32, "exited"));
    }
    let list = store.list_recent(Some(2));
    assert_eq!(list.len(), 2);
}

#[test]
fn save_and_load_roundtrip() {
    let _dir = TempDir::new().unwrap();
    // We can't easily override the path in unit tests without env manipulation,
    // so just verify serialization roundtrip via JSON directly.
    let mut store = ProcessStore::default();
    store.add(make_entry("id-1", 42, "running"));
    let json = serde_json::to_string(&store).unwrap();
    let loaded: ProcessStore = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.processes.len(), 1);
    assert_eq!(loaded.find("id-1").unwrap().pid, 42);
}

#[test]
fn update_status_nonexistent_id() {
    let mut store = ProcessStore::default();
    // Should be a no-op, not panic
    store.update_status("nonexistent", "exited", Some(1));
    assert!(store.find("nonexistent").is_none());
}

#[test]
fn list_recent_empty_store() {
    let store = ProcessStore::default();
    let list = store.list_recent(None);
    assert!(list.is_empty());
}

#[test]
fn find_empty_store() {
    let store = ProcessStore::default();
    assert!(store.find("any-id").is_none());
}

#[test]
fn process_entry_full_serialization_roundtrip() {
    let entry = ProcessEntry {
        id: "id-full".to_string(),
        pid: 12345,
        session_id: Some("sess-abc".to_string()),
        provider: "claude".to_string(),
        model: "opus".to_string(),
        command: "exec".to_string(),
        prompt: Some("write a hello world program".to_string()),
        started_at: "2026-03-28T10:00:00Z".to_string(),
        status: "exited".to_string(),
        exit_code: Some(0),
        exited_at: Some("2026-03-28T10:05:00Z".to_string()),
        root: Some("/home/user/project".to_string()),
        parent_process_id: None,
        parent_session_id: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: ProcessEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "id-full");
    assert_eq!(parsed.pid, 12345);
    assert_eq!(parsed.session_id, Some("sess-abc".to_string()));
    assert_eq!(parsed.exit_code, Some(0));
    assert_eq!(parsed.exited_at, Some("2026-03-28T10:05:00Z".to_string()));
    assert_eq!(parsed.root, Some("/home/user/project".to_string()));
}

#[test]
fn process_entry_minimal_serialization_roundtrip() {
    let entry = ProcessEntry {
        id: "id-min".to_string(),
        pid: 1,
        session_id: None,
        provider: "codex".to_string(),
        model: "gpt-5.4".to_string(),
        command: "run".to_string(),
        prompt: None,
        started_at: "2026-03-28T10:00:00Z".to_string(),
        status: "running".to_string(),
        exit_code: None,
        exited_at: None,
        root: None,
        parent_process_id: None,
        parent_session_id: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: ProcessEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "id-min");
    assert!(parsed.session_id.is_none());
    assert!(parsed.prompt.is_none());
    assert!(parsed.exit_code.is_none());
    assert!(parsed.exited_at.is_none());
    assert!(parsed.root.is_none());
}
