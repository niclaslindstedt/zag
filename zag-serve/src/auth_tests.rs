use crate::auth::ServerState;
use crate::router::build_router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// Build a test app in legacy single-token mode.
fn test_app() -> axum::Router {
    build_router(ServerState {
        token: Some("test-token-123".to_string()),
        user_store: None,
        token_store: None,
        force_sandbox: false,
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
