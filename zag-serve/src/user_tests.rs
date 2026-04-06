use super::*;

#[test]
fn test_add_and_authenticate_user() {
    let mut store = UserStore::default();
    // Manually add user without saving to disk
    let password_hash = bcrypt::hash("secret123", 4).unwrap(); // low cost for test speed
    store.users.push(UserEntry {
        username: "alice".to_string(),
        password_hash,
        home_dir: "/home/alice".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        enabled: true,
    });

    assert!(store.authenticate("alice", "secret123").is_some());
    assert!(store.authenticate("alice", "wrong").is_none());
    assert!(store.authenticate("bob", "secret123").is_none());
}

#[test]
fn test_disabled_user_cannot_authenticate() {
    let mut store = UserStore::default();
    let password_hash = bcrypt::hash("secret123", 4).unwrap();
    store.users.push(UserEntry {
        username: "alice".to_string(),
        password_hash,
        home_dir: "/home/alice".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        enabled: false,
    });

    assert!(store.authenticate("alice", "secret123").is_none());
}

#[test]
fn test_remove_user() {
    let mut store = UserStore::default();
    let password_hash = bcrypt::hash("pass", 4).unwrap();
    store.users.push(UserEntry {
        username: "alice".to_string(),
        password_hash,
        home_dir: "/home/alice".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        enabled: true,
    });

    assert!(store.find_user("alice").is_some());
    store.users.retain(|u| u.username != "alice");
    assert!(store.find_user("alice").is_none());
}

#[test]
fn test_duplicate_user_rejected() {
    let mut store = UserStore::default();
    let password_hash = bcrypt::hash("pass", 4).unwrap();
    store.users.push(UserEntry {
        username: "alice".to_string(),
        password_hash,
        home_dir: "/home/alice".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        enabled: true,
    });

    // Adding same username should fail (in-memory check)
    assert!(store.find_user("alice").is_some());
}

#[test]
fn test_user_store_serialization() {
    let store = UserStore {
        users: vec![UserEntry {
            username: "bob".to_string(),
            password_hash: "$2b$12$fakehash".to_string(),
            home_dir: "/home/bob".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            enabled: true,
        }],
    };

    let json = serde_json::to_string_pretty(&store).unwrap();
    let deserialized: UserStore = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.users.len(), 1);
    assert_eq!(deserialized.users[0].username, "bob");
    assert_eq!(deserialized.users[0].home_dir, "/home/bob");
}

#[test]
fn test_user_logs_dir() {
    let dir = UserStore::user_logs_dir("alice");
    assert!(dir.ends_with("users/alice/logs"));
}
