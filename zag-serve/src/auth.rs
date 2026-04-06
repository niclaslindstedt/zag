//! Authentication middleware for the zag server.
//!
//! Supports two modes:
//! - **User accounts** (if `users.json` exists): Bearer tokens issued via `/api/v1/login`
//! - **Legacy single token** (fallback): shared `Authorization: Bearer <token>`

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::session_token::TokenStore;
use crate::user::UserStore;

/// Shared server state containing auth configuration.
#[derive(Clone)]
pub struct ServerState {
    /// Legacy single shared token (used when no users.json exists).
    pub token: Option<String>,
    pub user_store: Option<Arc<UserStore>>,
    pub token_store: Option<Arc<RwLock<TokenStore>>>,
    /// When true, all connected users' agent sessions are forced to run inside a Docker sandbox.
    pub force_sandbox: bool,
}

/// User context attached to requests when user-account mode is active.
/// Extracted from request extensions in handlers.
#[derive(Debug, Clone)]
pub struct UserContext {
    pub username: String,
    pub home_dir: PathBuf,
}

/// Marker inserted into request extensions when the legacy token is used.
/// Legacy tokens act as "super tokens" — they bypass user-account restrictions
/// and can perform administrative operations like user management.
#[derive(Debug, Clone)]
pub struct LegacyTokenContext;

/// Middleware that validates authentication.
///
/// Skips auth for `/api/v1/health` and `/api/v1/login`.
/// In user-account mode: validates session tokens and attaches `UserContext`.
/// In legacy mode: validates the shared bearer token.
pub async fn auth_middleware(
    state: axum::extract::State<ServerState>,
    request: Request,
    next: Next,
) -> Response {
    // Skip auth for health and login endpoints
    let path = request.uri().path();
    if path == "/api/v1/health" || path == "/api/v1/login" {
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = header[7..].to_string();

            // User-account mode: validate session token and attach UserContext
            if let Some(ref token_store) = state.token_store {
                let ts = token_store.read().await;
                if let Some(username) = ts.validate(&token) {
                    let username = username.to_string();
                    drop(ts);
                    if let Some(ref user_store) = state.user_store {
                        if let Some(user) = user_store.find_user(&username) {
                            let ctx = UserContext {
                                username,
                                home_dir: PathBuf::from(&user.home_dir),
                            };
                            let mut request = request;
                            request.extensions_mut().insert(ctx);
                            return next.run(request).await;
                        }
                    }
                }
                // Fall through to check legacy token (super token) before rejecting
            }

            // Legacy token: acts as a super token in both legacy-only and
            // user-account modes. Grants full access without user restrictions.
            if let Some(ref expected_token) = state.token {
                if token == *expected_token {
                    let mut request = request;
                    request.extensions_mut().insert(LegacyTokenContext);
                    return next.run(request).await;
                }
            }

            (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            "Missing or invalid Authorization header",
        )
            .into_response(),
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
