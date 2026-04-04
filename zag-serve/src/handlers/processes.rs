//! Process management HTTP handlers.

use axum::{Json, extract::Query, http::StatusCode, response::IntoResponse};

use crate::types::*;

/// GET /api/v1/processes
pub async fn list(Query(query): Query<ProcessListQuery>) -> impl IntoResponse {
    match zag_orch::ps::list_processes(
        query.running.unwrap_or(false),
        query.limit,
        query.provider.as_deref(),
    ) {
        Ok(processes) => Json(serde_json::to_value(&processes).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
