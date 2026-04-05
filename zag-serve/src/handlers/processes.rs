//! Process management HTTP handlers.

use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
};

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

/// GET /api/v1/processes/:id
pub async fn show(Path(id): Path<String>) -> impl IntoResponse {
    match zag_orch::ps::get_process(&id) {
        Ok(info) => Json(serde_json::to_value(&info).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/processes/:id/stop
pub async fn stop(Path(id): Path<String>) -> impl IntoResponse {
    match zag_orch::ps::request_stop(&id) {
        Ok(()) => Json(serde_json::json!({"status": "stopped"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/processes/:id/kill
pub async fn kill(Path(id): Path<String>) -> impl IntoResponse {
    match zag_orch::ps::request_kill(&id) {
        Ok(()) => Json(serde_json::json!({"status": "killed"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
