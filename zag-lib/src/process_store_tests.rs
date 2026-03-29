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
