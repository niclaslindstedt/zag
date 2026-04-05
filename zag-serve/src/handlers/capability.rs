//! Capability HTTP handler.

use axum::{Json, extract::Query, http::StatusCode, response::IntoResponse};

use crate::types::*;

/// GET /api/v1/capability
pub async fn capability(Query(query): Query<CapabilityQuery>) -> impl IntoResponse {
    let provider = query.provider.as_deref().unwrap_or("claude");
    let format = query.format.as_deref().unwrap_or("json");
    let pretty = query.pretty.unwrap_or(false);

    match zag_agent::capability::get_capability(provider) {
        Ok(cap) => match zag_agent::capability::format_capability(&cap, format, pretty) {
            Ok(output) => {
                if format == "json" {
                    // Parse and return as JSON value for proper content-type
                    match serde_json::from_str::<serde_json::Value>(&output) {
                        Ok(v) => Json(v).into_response(),
                        Err(_) => output.into_response(),
                    }
                } else {
                    output.into_response()
                }
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
