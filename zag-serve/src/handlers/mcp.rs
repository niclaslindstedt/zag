//! MCP server management HTTP handler.

use std::collections::BTreeMap;

use axum::{Json, http::StatusCode, response::IntoResponse};

use crate::types::*;

/// POST /api/v1/mcp
pub async fn mcp(Json(req): Json<McpRequest>) -> impl IntoResponse {
    let root = req.root.as_deref();

    match req.command.as_str() {
        "list" => match zag_agent::mcp::list_servers(root) {
            Ok(servers) => Json(serde_json::to_value(&servers).unwrap_or_default()).into_response(),
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
            match zag_agent::mcp::get_server(name, root) {
                Ok(server) => {
                    Json(serde_json::to_value(&server).unwrap_or_default()).into_response()
                }
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
                Some(ref n) => n.clone(),
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
            let transport = req.transport.as_deref().unwrap_or("stdio");
            let server = zag_agent::mcp::McpServer {
                name,
                transport: transport.to_string(),
                command: req.server_command.clone(),
                args: req.args.clone().unwrap_or_default(),
                url: req.url.clone(),
                env: parse_env_pairs(&req.env.clone().unwrap_or_default()),
                description: req.description.clone().unwrap_or_default(),
                bearer_token_env_var: None,
                headers: Default::default(),
            };
            let is_global = req.global.unwrap_or(false);
            match zag_agent::mcp::add_server(&server, is_global, root) {
                Ok(path) => Json(serde_json::json!({
                    "status": "added",
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
            match zag_agent::mcp::remove_server(name, root) {
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
            let all_servers = match zag_agent::mcp::load_all_servers(root) {
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
            let providers: Vec<&str> = if let Some(ref p) = req.from {
                vec![p.as_str()]
            } else {
                vec!["claude", "codex", "gemini", "copilot"]
            };
            let mut total = 0;
            for p in &providers {
                if let Ok(n) = zag_agent::mcp::sync_servers_for_provider(p, &all_servers) {
                    total += n;
                }
            }
            Json(serde_json::json!({"status": "synced", "count": total})).into_response()
        }
        "import" => {
            let from = req.from.as_deref().unwrap_or("claude");
            match zag_agent::mcp::import_servers(from) {
                Ok(names) => Json(serde_json::json!({
                    "status": "imported",
                    "servers": names,
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
                error: format!("Unknown mcp command: {}", req.command),
            }),
        )
            .into_response(),
    }
}

/// Parse "KEY=VALUE" pairs into a BTreeMap.
fn parse_env_pairs(pairs: &[String]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for pair in pairs {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}
