//! Configuration HTTP handler.

use axum::{Json, http::StatusCode, response::IntoResponse};

use crate::types::*;

/// POST /api/v1/config
pub async fn config(Json(req): Json<ConfigRequest>) -> impl IntoResponse {
    let root = req.root.as_deref();
    let args = &req.args;

    if args.is_empty() {
        // Return full config
        let path = zag_agent::config::Config::config_path(root);
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    return Json(serde_json::json!({
                        "path": path.display().to_string(),
                        "content": content,
                    }))
                    .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            }
        } else {
            return Json(serde_json::json!({
                "path": path.display().to_string(),
                "content": null,
            }))
            .into_response();
        }
    }

    if args.len() == 1 && args[0] == "init" {
        match zag_agent::config::Config::init(root) {
            Ok(created) => {
                let path = zag_agent::config::Config::config_path(root);
                return Json(serde_json::json!({
                    "created": created,
                    "path": path.display().to_string(),
                }))
                .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    if args.len() == 1 && args[0] == "reset" {
        let path = zag_agent::config::Config::config_path(root);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        match zag_agent::config::Config::init(root) {
            Ok(_) => {
                return Json(serde_json::json!({
                    "status": "reset",
                    "path": path.display().to_string(),
                }))
                .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    if args.len() == 1 && args[0] == "path" {
        let path = zag_agent::config::Config::config_path(root);
        return Json(serde_json::json!({"path": path.display().to_string()})).into_response();
    }

    if args.len() == 1 && args[0] == "list" {
        let config = zag_agent::config::Config::load(root).unwrap_or_default();
        let mut map = serde_json::Map::new();
        for key in zag_agent::config::Config::VALID_KEYS {
            let value = config.get_value(key);
            map.insert(
                key.to_string(),
                value
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null),
            );
        }
        return Json(serde_json::Value::Object(map)).into_response();
    }

    if args.len() == 2 && args[0] == "unset" {
        let mut config = zag_agent::config::Config::load(root).unwrap_or_default();
        match config.unset_value(&args[1]) {
            Ok(()) => match config.save(root) {
                Ok(()) => {
                    return Json(serde_json::json!({"key": args[1], "status": "unset"}))
                        .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            },
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response();
            }
        }
    }

    if args.len() == 2 && args[0] == "get" {
        let config = zag_agent::config::Config::load(root).unwrap_or_default();
        let value = config.get_value(&args[1]);
        return Json(serde_json::json!({
            "key": args[1],
            "value": value,
        }))
        .into_response();
    }

    // Parse key=value or key value
    let (key, value) = if args.len() == 1 {
        if let Some((k, v)) = args[0].split_once('=') {
            (k.to_string(), Some(v.to_string()))
        } else {
            // Implicit get
            let config = zag_agent::config::Config::load(root).unwrap_or_default();
            let val = config.get_value(&args[0]);
            return Json(serde_json::json!({
                "key": args[0],
                "value": val,
            }))
            .into_response();
        }
    } else if args.len() == 2 {
        (args[0].clone(), Some(args[1].clone()))
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid config arguments".to_string(),
            }),
        )
            .into_response();
    };

    if let Some(val) = value {
        let mut config = zag_agent::config::Config::load(root).unwrap_or_default();
        match config.set_value(&key, &val) {
            Ok(()) => match config.save(root) {
                Ok(()) => Json(serde_json::json!({"key": key, "value": val})).into_response(),
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
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing value".to_string(),
            }),
        )
            .into_response()
    }
}
