//! User management HTTP handlers.
//!
//! These endpoints require the legacy (super) token. Regular session tokens
//! cannot manage user accounts.

use axum::{
    Json,
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::auth::LegacyTokenContext;
use crate::types::*;
use crate::user::UserStore;

/// Guard that rejects requests not authenticated with the legacy (super) token.
fn require_super_token(ctx: &Option<Extension<LegacyTokenContext>>) -> Option<Response> {
    if ctx.is_none() {
        Some(
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "User management requires the legacy (super) token".to_string(),
                }),
            )
                .into_response(),
        )
    } else {
        None
    }
}

/// POST /api/v1/users/add
///
/// Creates a new user account. Requires the legacy (super) token.
pub async fn add(
    super_token: Option<Extension<LegacyTokenContext>>,
    Json(req): Json<UserAddRequest>,
) -> impl IntoResponse {
    if let Some(resp) = require_super_token(&super_token) {
        return resp;
    }

    if req.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password cannot be empty".to_string(),
            }),
        )
            .into_response();
    }

    let mut store = match UserStore::load() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load user store: {}", e),
                }),
            )
                .into_response();
        }
    };

    if let Err(e) = store.add_user(&req.username, &req.password, &req.home_dir) {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response();
    }

    // Create directories for the new user
    let home_path = std::path::Path::new(&req.home_dir);
    if !home_path.exists() {
        let _ = std::fs::create_dir_all(home_path);
    }
    let logs_dir = UserStore::user_logs_dir(&req.username);
    let _ = std::fs::create_dir_all(&logs_dir);

    (
        StatusCode::CREATED,
        Json(UserResponse {
            message: format!("User '{}' created successfully", req.username),
        }),
    )
        .into_response()
}

/// GET /api/v1/users
///
/// Lists all user accounts. Requires the legacy (super) token.
pub async fn list(super_token: Option<Extension<LegacyTokenContext>>) -> impl IntoResponse {
    if let Some(resp) = require_super_token(&super_token) {
        return resp;
    }

    let store = match UserStore::load() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load user store: {}", e),
                }),
            )
                .into_response();
        }
    };

    let entries: Vec<UserListEntry> = store
        .list_users()
        .iter()
        .map(|u| UserListEntry {
            username: u.username.clone(),
            home_dir: u.home_dir.clone(),
            enabled: u.enabled,
            created_at: u.created_at.clone(),
        })
        .collect();

    (StatusCode::OK, Json(entries)).into_response()
}

/// POST /api/v1/users/remove
///
/// Removes a user account. Requires the legacy (super) token.
pub async fn remove(
    super_token: Option<Extension<LegacyTokenContext>>,
    Json(req): Json<UserRemoveRequest>,
) -> impl IntoResponse {
    if let Some(resp) = require_super_token(&super_token) {
        return resp;
    }

    let mut store = match UserStore::load() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load user store: {}", e),
                }),
            )
                .into_response();
        }
    };

    if let Err(e) = store.remove_user(&req.username) {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(UserResponse {
            message: format!("User '{}' removed", req.username),
        }),
    )
        .into_response()
}

/// POST /api/v1/users/passwd
///
/// Changes a user's password. Requires the legacy (super) token.
pub async fn passwd(
    super_token: Option<Extension<LegacyTokenContext>>,
    Json(req): Json<UserPasswdRequest>,
) -> impl IntoResponse {
    if let Some(resp) = require_super_token(&super_token) {
        return resp;
    }

    if req.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Password cannot be empty".to_string(),
            }),
        )
            .into_response();
    }

    let mut store = match UserStore::load() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load user store: {}", e),
                }),
            )
                .into_response();
        }
    };

    if let Err(e) = store.change_password(&req.username, &req.password) {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(UserResponse {
            message: format!("Password changed for user '{}'", req.username),
        }),
    )
        .into_response()
}
