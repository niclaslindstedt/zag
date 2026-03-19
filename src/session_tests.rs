use super::*;

fn temp_root(suffix: &str) -> (String, impl Drop) {
    let dir = std::env::temp_dir().join(format!(
        "agent-session-test-{}-{}",
        std::process::id(),
        suffix
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".agent")).unwrap();
    let root = dir.to_str().unwrap().to_string();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    (root, Cleanup(dir))
}

fn sample_entry(id: &str) -> SessionEntry {
    SessionEntry {
        session_id: id.to_string(),
        provider: "claude".to_string(),
        worktree_path: "/tmp/test-wt".to_string(),
        worktree_name: "test-wt".to_string(),
        created_at: "2026-03-13T00:00:00Z".to_string(),
        sandbox_name: None,
    }
}

#[test]
fn test_load_missing_file_returns_empty() {
    let (root, _guard) = temp_root("empty");
    let store = SessionStore::load(Some(&root)).unwrap();
    assert!(store.sessions.is_empty());
}

#[test]
fn test_save_and_load_round_trip() {
    let (root, _guard) = temp_root("roundtrip");
    let mut store = SessionStore::default();
    store.add(sample_entry("abc-123"));
    store.add(sample_entry("def-456"));
    store.save(Some(&root)).unwrap();

    let loaded = SessionStore::load(Some(&root)).unwrap();
    assert_eq!(loaded.sessions.len(), 2);
    assert_eq!(loaded.sessions[0].session_id, "abc-123");
    assert_eq!(loaded.sessions[1].session_id, "def-456");
}

#[test]
fn test_find_by_session_id() {
    let mut store = SessionStore::default();
    store.add(sample_entry("abc-123"));
    store.add(sample_entry("def-456"));

    assert!(store.find_by_session_id("abc-123").is_some());
    assert_eq!(
        store.find_by_session_id("abc-123").unwrap().session_id,
        "abc-123"
    );
    assert!(store.find_by_session_id("nonexistent").is_none());
}

#[test]
fn test_remove() {
    let mut store = SessionStore::default();
    store.add(sample_entry("abc-123"));
    store.add(sample_entry("def-456"));

    store.remove("abc-123");
    assert_eq!(store.sessions.len(), 1);
    assert_eq!(store.sessions[0].session_id, "def-456");

    // Removing nonexistent is a no-op
    store.remove("nonexistent");
    assert_eq!(store.sessions.len(), 1);
}
