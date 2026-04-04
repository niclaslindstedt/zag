//! Bearer token authentication middleware for the zag server.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Shared server state containing the auth token.
#[derive(Clone)]
pub struct ServerState {
    pub token: String,
}

/// Middleware that validates the Authorization: Bearer <token> header.
pub async fn auth_middleware(
    state: axum::extract::State<ServerState>,
    request: Request,
    next: Next,
) -> Response {
    // Skip auth for health endpoint
    if request.uri().path() == "/api/v1/health" {
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if token == state.token {
                next.run(request).await
            } else {
                (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
            }
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
