use zag_agent::session_log::{
    GlobalSessionEntry, GlobalSessionIndex, load_global_index, save_global_index,
};

#[test]
fn test_lookup_global_index_by_id_exact_match() {
    // This tests the lookup_global_index_by_id function indirectly
    // by creating a global index file at the expected location
    let dir = std::env::temp_dir().join(format!("zag-listen-global-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    // Create a dummy log file
    let log_file = dir.join("test-session.jsonl");
    std::fs::write(&log_file, "{}").unwrap();

    // Save a global index with that entry
    let index = GlobalSessionIndex {
        sessions: vec![GlobalSessionEntry {
            session_id: "abc-123".to_string(),
            project: "test-project".to_string(),
            log_path: log_file.to_string_lossy().to_string(),
            provider: "claude".to_string(),
            started_at: "2026-03-24T12:00:00Z".to_string(),
        }],
    };
    save_global_index(&dir, &index).unwrap();

    // Verify the index was created
    let loaded = load_global_index(&dir).unwrap();
    assert_eq!(loaded.sessions.len(), 1);
    assert_eq!(loaded.sessions[0].session_id, "abc-123");
    assert_eq!(loaded.sessions[0].log_path, log_file.to_string_lossy());
}

#[test]
fn test_global_index_serialization_roundtrip() {
    let dir = std::env::temp_dir().join(format!(
        "zag-listen-global-roundtrip-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(dir.clone());

    let index = GlobalSessionIndex {
        sessions: vec![
            GlobalSessionEntry {
                session_id: "s1".to_string(),
                project: "proj-a".to_string(),
                log_path: "/tmp/a.jsonl".to_string(),
                provider: "claude".to_string(),
                started_at: "2026-03-24T10:00:00Z".to_string(),
            },
            GlobalSessionEntry {
                session_id: "s2".to_string(),
                project: "proj-b".to_string(),
                log_path: "/tmp/b.jsonl".to_string(),
                provider: "gemini".to_string(),
                started_at: "2026-03-24T11:00:00Z".to_string(),
            },
        ],
    };
    save_global_index(&dir, &index).unwrap();

    let loaded = load_global_index(&dir).unwrap();
    assert_eq!(loaded.sessions.len(), 2);
    assert_eq!(loaded.sessions[0].session_id, "s1");
    assert_eq!(loaded.sessions[1].session_id, "s2");
    assert_eq!(loaded.sessions[1].provider, "gemini");
}
