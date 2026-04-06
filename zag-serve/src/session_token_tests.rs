use super::*;

#[test]
fn test_create_and_validate_token() {
    let mut store = TokenStore::new();
    let token = store.create_token("alice");
    assert_eq!(store.validate(&token), Some("alice"));
}

#[test]
fn test_invalid_token_returns_none() {
    let store = TokenStore::new();
    assert_eq!(store.validate("nonexistent"), None);
}

#[test]
fn test_revoke_token() {
    let mut store = TokenStore::new();
    let token = store.create_token("alice");
    assert!(store.validate(&token).is_some());
    store.revoke(&token);
    assert!(store.validate(&token).is_none());
}

#[test]
fn test_expired_token_returns_none() {
    let mut store = TokenStore {
        tokens: HashMap::new(),
        token_lifetime_hours: 0, // expires immediately
    };
    let token = crate::generate_token();
    let now = chrono::Utc::now();
    store.tokens.insert(
        token.clone(),
        TokenEntry {
            username: "alice".to_string(),
            created_at: now - chrono::Duration::hours(2),
            expires_at: now - chrono::Duration::hours(1), // already expired
        },
    );
    assert!(store.validate(&token).is_none());
}

#[test]
fn test_cleanup_expired() {
    let mut store = TokenStore::new();
    let valid_token = store.create_token("alice");

    // Insert an expired token manually
    let expired_token = "expired_token_123".to_string();
    let now = chrono::Utc::now();
    store.tokens.insert(
        expired_token.clone(),
        TokenEntry {
            username: "bob".to_string(),
            created_at: now - chrono::Duration::hours(48),
            expires_at: now - chrono::Duration::hours(24),
        },
    );

    assert_eq!(store.tokens.len(), 2);
    store.cleanup_expired();
    assert_eq!(store.tokens.len(), 1);
    assert!(store.validate(&valid_token).is_some());
    assert!(store.validate(&expired_token).is_none());
}
