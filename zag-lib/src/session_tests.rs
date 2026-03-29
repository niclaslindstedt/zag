use super::*;

fn temp_root(suffix: &str) -> (String, impl Drop) {
    let dir = std::env::temp_dir().join(format!(
        "zag-session-test-{}-{}",
        std::process::id(),
        suffix
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
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
        model: "opus".to_string(),
        worktree_path: "/tmp/test-wt".to_string(),
        worktree_name: "test-wt".to_string(),
        created_at: "2026-03-13T00:00:00Z".to_string(),
        provider_session_id: None,
        sandbox_name: None,
        is_worktree: true,
        discovered: false,
        discovery_source: None,
        log_path: None,
        log_completeness: "partial".to_string(),
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
fn test_find_by_provider_session_id() {
    let mut store = SessionStore::default();
    let mut entry = sample_entry("abc-123");
    entry.provider_session_id = Some("native-1".to_string());
    store.add(entry);

    assert!(store.find_by_provider_session_id("native-1").is_some());
    assert_eq!(
        store
            .find_by_provider_session_id("native-1")
            .unwrap()
            .session_id,
        "abc-123"
    );
    assert!(store.find_by_provider_session_id("missing").is_none());
}

#[test]
fn test_find_by_any_id_matches_wrapper_or_provider_id() {
    let mut store = SessionStore::default();
    let mut entry = sample_entry("abc-123");
    entry.provider_session_id = Some("native-1".to_string());
    store.add(entry);

    assert_eq!(
        store.find_by_any_id("abc-123").unwrap().session_id,
        "abc-123"
    );
    assert_eq!(
        store.find_by_any_id("native-1").unwrap().session_id,
        "abc-123"
    );
}

#[test]
fn test_latest_returns_most_recent_session() {
    let mut store = SessionStore::default();
    let mut older = sample_entry("abc-123");
    older.created_at = "2026-03-13T00:00:00Z".to_string();
    let mut newer = sample_entry("def-456");
    newer.created_at = "2026-03-14T00:00:00Z".to_string();
    store.add(older);
    store.add(newer);

    assert_eq!(store.latest().unwrap().session_id, "def-456");
}

#[test]
fn test_set_provider_session_id() {
    let mut store = SessionStore::default();
    store.add(sample_entry("abc-123"));

    store.set_provider_session_id("abc-123", "native-1".to_string());

    assert_eq!(
        store
            .find_by_session_id("abc-123")
            .unwrap()
            .provider_session_id
            .as_deref(),
        Some("native-1")
    );
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

#[test]
fn test_list_returns_all_sorted() {
    let mut store = SessionStore::default();
    let mut e1 = sample_entry("aaa");
    e1.created_at = "2026-03-10T00:00:00Z".to_string();
    let mut e2 = sample_entry("bbb");
    e2.created_at = "2026-03-15T00:00:00Z".to_string();
    let mut e3 = sample_entry("ccc");
    e3.created_at = "2026-03-12T00:00:00Z".to_string();
    store.add(e1);
    store.add(e2);
    store.add(e3);

    let infos = store.list();
    assert_eq!(infos.len(), 3);
    // Should be sorted by created_at descending (newest first)
    assert_eq!(infos[0].session_id, "bbb");
    assert_eq!(infos[1].session_id, "ccc");
    assert_eq!(infos[2].session_id, "aaa");
}

#[test]
fn test_list_empty_store() {
    let store = SessionStore::default();
    let infos = store.list();
    assert!(infos.is_empty());
}

#[test]
fn test_get_by_session_id() {
    let mut store = SessionStore::default();
    store.add(sample_entry("abc-123"));
    store.add(sample_entry("def-456"));

    let info = store.get("abc-123");
    assert!(info.is_some());
    assert_eq!(info.unwrap().session_id, "abc-123");

    assert!(store.get("nonexistent").is_none());
}

#[test]
fn test_get_by_provider_session_id() {
    let mut store = SessionStore::default();
    let mut entry = sample_entry("wrapper-1");
    entry.provider_session_id = Some("native-abc".to_string());
    store.add(entry);

    let info = store.get("native-abc");
    assert!(info.is_some());
    assert_eq!(info.unwrap().session_id, "wrapper-1");
}
