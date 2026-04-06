use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::ServerState;
use crate::router::build_router;
use crate::session_token::TokenStore;
use crate::user::{UserEntry, UserStore};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// Build a test app in legacy single-token mode.
fn test_app() -> axum::Router {
    build_router(ServerState {
        token: Some("test-token-123".to_string()),
        user_store: None,
        token_store: None,
    })
}

/// Build a test app in user-account mode WITH a legacy token configured.
fn test_app_user_mode_with_legacy_token() -> axum::Router {
    let user_store = UserStore {
        users: vec![UserEntry {
            username: "alice".to_string(),
            password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
            home_dir: "/home/alice".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            enabled: true,
        }],
    };
    build_router(ServerState {
        token: Some("super-token-456".to_string()),
        user_store: Some(Arc::new(user_store)),
        token_store: Some(Arc::new(RwLock::new(TokenStore::new()))),
    })
}

#[tokio::test]
async fn health_no_auth_required() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/v1/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn missing_auth_returns_401() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/v1/sessions")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_token_returns_401() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/v1/sessions")
        .header("Authorization", "Bearer wrong-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_token_passes_auth() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/v1/sessions")
        .header("Authorization", "Bearer test-token-123")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // Should not be 401 — may be 200 or 500 depending on session store state
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_endpoint_skips_auth() {
    let app = test_app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"test","password":"test"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // Should not be 401 (auth is skipped); may be 400 or 404 depending on mode
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn legacy_token_passes_auth_in_user_account_mode() {
    let app = test_app_user_mode_with_legacy_token();
    let req = Request::builder()
        .uri("/api/v1/sessions")
        .header("Authorization", "Bearer super-token-456")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // Should not be 401 — legacy token acts as super token
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_token_rejected_in_user_account_mode() {
    let app = test_app_user_mode_with_legacy_token();
    let req = Request::builder()
        .uri("/api/v1/sessions")
        .header("Authorization", "Bearer wrong-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn session_token_still_works_in_user_account_mode() {
    let user_store = UserStore {
        users: vec![UserEntry {
            username: "alice".to_string(),
            password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
            home_dir: "/home/alice".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            enabled: true,
        }],
    };
    let token_store = Arc::new(RwLock::new(TokenStore::new()));
    let session_token = token_store.write().await.create_token("alice");

    let app = build_router(ServerState {
        token: Some("super-token-456".to_string()),
        user_store: Some(Arc::new(user_store)),
        token_store: Some(token_store),
    });

    let req = Request::builder()
        .uri("/api/v1/sessions")
        .header("Authorization", format!("Bearer {}", session_token))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // Session token should still work (not 401)
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}
