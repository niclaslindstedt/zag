use crate::auth::ServerState;
use crate::router::build_router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn test_app() -> axum::Router {
    build_router(ServerState {
        token: "test-token-123".to_string(),
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
