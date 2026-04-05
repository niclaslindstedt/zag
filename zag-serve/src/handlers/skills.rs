//! Skills HTTP handler.

use axum::{Json, http::StatusCode, response::IntoResponse};

use crate::types::*;

/// POST /api/v1/skills
pub async fn skills(Json(req): Json<SkillsRequest>) -> impl IntoResponse {
    match req.command.as_str() {
        "list" => match zag_agent::skills::list_skills() {
            Ok(skills) => Json(serde_json::to_value(&skills).unwrap_or_default()).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        "show" => {
            let name = match req.name {
                Some(ref n) => n,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "name is required for show".to_string(),
                        }),
                    )
                        .into_response();
                }
            };
            match zag_agent::skills::get_skill(name) {
                Ok(skill) => Json(serde_json::to_value(&skill).unwrap_or_default()).into_response(),
                Err(e) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
        "add" => {
            let name = match req.name {
                Some(ref n) => n,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "name is required for add".to_string(),
                        }),
                    )
                        .into_response();
                }
            };
            let desc = req.description.as_deref().unwrap_or("");
            match zag_agent::skills::add_skill(name, desc) {
                Ok(path) => Json(serde_json::json!({
                    "status": "created",
                    "path": path.display().to_string(),
                }))
                .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
        "remove" => {
            let name = match req.name {
                Some(ref n) => n,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error: "name is required for remove".to_string(),
                        }),
                    )
                        .into_response();
                }
            };
            match zag_agent::skills::remove_skill(name) {
                Ok(()) => Json(serde_json::json!({"status": "removed"})).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
        "sync" => {
            let all_skills = match zag_agent::skills::load_all_skills() {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            };
            let providers: Vec<&str> = if let Some(ref p) = req.provider {
                vec![p.as_str()]
            } else {
                vec!["claude", "codex", "gemini", "copilot"]
            };
            let mut total = 0;
            for p in &providers {
                if let Ok(n) = zag_agent::skills::sync_skills_for_provider(p, &all_skills) {
                    total += n;
                }
            }
            Json(serde_json::json!({"status": "synced", "count": total})).into_response()
        }
        "import" => {
            let from = req.from.as_deref().unwrap_or("claude");
            match zag_agent::skills::import_skills(from) {
                Ok(names) => Json(serde_json::json!({
                    "status": "imported",
                    "skills": names,
                }))
                .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Unknown skills command: {}", req.command),
            }),
        )
            .into_response(),
    }
}
