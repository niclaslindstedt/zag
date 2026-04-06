//! Login and logout HTTP handlers.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::auth::ServerState;
use crate::types::*;

/// POST /api/v1/login
///
/// Authenticates a user with username/password and returns a session token.
/// Only available when user-account mode is active (users.json exists).
pub async fn login(
    State(state): State<ServerState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let user_store = match state.user_store {
        Some(ref store) => store,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "User accounts are not enabled on this server".to_string(),
                }),
            )
                .into_response();
        }
    };

    let token_store = match state.token_store {
        Some(ref store) => store,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Token store not available".to_string(),
                }),
            )
                .into_response();
        }
    };

    match user_store.authenticate(&req.username, &req.password) {
        Some(user) => {
            let token = token_store.write().await.create_token(&req.username);
            (
                StatusCode::OK,
                Json(LoginResponse {
                    token,
                    username: user.username.clone(),
                    home_dir: user.home_dir.clone(),
                }),
            )
                .into_response()
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid username or password".to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/logout
///
/// Revokes the current session token.
pub async fn logout(
    State(state): State<ServerState>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    let token_store = match state.token_store {
        Some(ref store) => store,
        None => {
            return (
                StatusCode::OK,
                Json(LogoutResponse {
                    message: "No session to revoke".to_string(),
                }),
            )
                .into_response();
        }
    };

    // Extract the bearer token from the Authorization header
    if let Some(auth_header) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token_store.write().await.revoke(token);
        }
    }

    (
        StatusCode::OK,
        Json(LogoutResponse {
            message: "Logged out successfully".to_string(),
        }),
    )
        .into_response()
}
