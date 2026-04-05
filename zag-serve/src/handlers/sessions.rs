//! Session management HTTP handlers.

use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
};

use crate::types::*;

/// Send a message to an interactive session's FIFO.
async fn send_via_fifo(fifo: &std::path::Path, message: &str) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let ndjson = serde_json::json!({
        "type": "user_message",
        "content": message,
    });
    let line = format!("{}\n", serde_json::to_string(&ndjson)?);
    let mut file = tokio::fs::OpenOptions::new().write(true).open(fifo).await?;
    file.write_all(line.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}

/// POST /api/v1/sessions/spawn
pub async fn spawn(Json(req): Json<SpawnRequest>) -> impl IntoResponse {
    let provider = req.provider.unwrap_or_else(|| {
        zag_agent::config::resolve_provider(None, req.root.as_deref())
            .unwrap_or_else(|_| "claude".to_string())
    });

    let interactive = req.interactive.unwrap_or(false);

    if req.prompt.is_none() && !interactive {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "A prompt is required unless interactive is set".to_string(),
            }),
        )
            .into_response();
    }

    let params = zag_orch::spawn::SpawnParams {
        prompt: req.prompt,
        provider,
        model: req.model,
        root: req.root,
        auto_approve: req.auto_approve.unwrap_or(false),
        system_prompt: req.system_prompt,
        add_dirs: req.add_dirs.unwrap_or_default(),
        size: req.size,
        max_turns: req.max_turns,
        json: true,
        metadata: zag_orch::types::SessionMetadata {
            name: req.name,
            description: req.description,
            tags: req.tags.unwrap_or_default(),
        },
        depends_on: req.depends_on.unwrap_or_default(),
        inject_context: req.inject_context.unwrap_or(false),
        retried_from: None,
        interactive,
    };

    match zag_orch::spawn::spawn_session(&params) {
        Ok(result) => (
            StatusCode::CREATED,
            Json(SpawnResponse {
                session_id: result.session_id,
                pid: result.pid,
                log_path: result.log_path,
                interactive: result.interactive,
            }),
        )
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

/// GET /api/v1/sessions
pub async fn list(Query(query): Query<SessionListQuery>) -> impl IntoResponse {
    let store_result = if query.global.unwrap_or(false) {
        zag_agent::session::SessionStore::load_all()
    } else {
        zag_agent::session::SessionStore::load(None)
    };

    match store_result {
        Ok(store) => {
            let mut sessions = if let Some(ref tag) = query.tag {
                store.find_by_tag(tag).into_iter().cloned().collect()
            } else {
                store.sessions.clone()
            };

            if let Some(ref provider) = query.provider {
                sessions.retain(|s| s.provider == *provider);
            }

            if let Some(limit) = query.limit {
                sessions.truncate(limit);
            }

            Json(serde_json::to_value(&sessions).unwrap_or_default()).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/sessions/:id
pub async fn show(Path(id): Path<String>) -> impl IntoResponse {
    let store = match zag_agent::session::SessionStore::load(None) {
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

    match store.find_by_any_id(&id) {
        Some(entry) => Json(serde_json::to_value(entry).unwrap_or_default()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/sessions/:id/status
pub async fn status(Path(id): Path<String>) -> impl IntoResponse {
    match zag_orch::status::determine_status(&id, None) {
        Ok(info) => Json(serde_json::to_value(&info).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/sessions/:id/events
pub async fn events(Path(id): Path<String>, Query(query): Query<EventsQuery>) -> impl IntoResponse {
    let params = zag_orch::events::EventsParams {
        session_id: id,
        event_type: query.event_type,
        last: query.last,
        after_seq: query.after_seq,
        before_seq: query.before_seq,
        count: false,
        json: true,
        root: None,
    };

    match zag_orch::events::read_events(&params) {
        Ok(events) => Json(serde_json::to_value(&events).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/sessions/:id/output
pub async fn output(Path(id): Path<String>) -> impl IntoResponse {
    match zag_orch::collect::extract_last_assistant_message(&id, None) {
        Some(text) => Json(serde_json::json!({
            "session_id": id,
            "result": text,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("No output found for session {}", id),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/:id/cancel
pub async fn cancel(Path(id): Path<String>, Json(req): Json<CancelRequest>) -> impl IntoResponse {
    let params = zag_orch::cancel::CancelParams {
        session_ids: vec![id],
        tag: None,
        reason: req.reason,
        json: true,
        root: None,
    };

    match zag_orch::cancel::run_cancel(params) {
        Ok(()) => Json(serde_json::json!({"status": "cancelled"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/collect
pub async fn collect(Json(req): Json<CollectRequest>) -> impl IntoResponse {
    let params = zag_orch::collect::CollectParams {
        session_ids: req.session_ids,
        tag: req.tag,
        json: true,
        root: None,
    };

    match zag_orch::collect::collect_results(&params) {
        Ok(results) => Json(serde_json::to_value(&results).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/wait
pub async fn wait(Json(req): Json<WaitRequest>) -> impl IntoResponse {
    let params = zag_orch::wait::WaitParams {
        session_ids: req.session_ids,
        tag: req.tag,
        latest: false,
        timeout: req.timeout,
        any: req.any.unwrap_or(false),
        json: true,
        root: None,
    };

    // Run in a blocking task since wait_for_sessions uses thread::sleep
    match tokio::task::spawn_blocking(move || zag_orch::wait::wait_for_sessions(&params)).await {
        Ok(Ok(results)) => Json(serde_json::to_value(&results).unwrap_or_default()).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
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

/// DELETE /api/v1/sessions/:id
pub async fn delete(Path(id): Path<String>) -> impl IntoResponse {
    let mut store = match zag_agent::session::SessionStore::load(None) {
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

    if store.get(&id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", id),
            }),
        )
            .into_response();
    }

    store.remove(&id);
    match store.save(None) {
        Ok(()) => Json(serde_json::json!({"deleted": id})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// PATCH /api/v1/sessions/:id
pub async fn update(
    Path(id): Path<String>,
    Json(req): Json<crate::types::SessionUpdateRequest>,
) -> impl IntoResponse {
    let mut store = match zag_agent::session::SessionStore::load(None) {
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

    let entry = store.sessions.iter_mut().find(|e| e.session_id == id);
    let entry = match entry {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                }),
            )
                .into_response();
        }
    };

    if req.name.is_some() {
        entry.name = req.name;
    }
    if req.description.is_some() {
        entry.description = req.description;
    }
    if req.clear_tags.unwrap_or(false) {
        entry.tags.clear();
    }
    if let Some(tags) = req.tags {
        entry.tags.extend(tags);
    }

    let updated = serde_json::to_value(&*entry).unwrap_or_default();

    match store.save(None) {
        Ok(()) => Json(updated).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/:id/input
pub async fn input(Path(id): Path<String>, Json(req): Json<InputRequest>) -> impl IntoResponse {
    // Check if this is an interactive session with a FIFO
    let fifo = zag_orch::spawn::fifo_path(&id);
    if fifo.exists() {
        return match send_via_fifo(&fifo, &req.message).await {
            Ok(()) => {
                Json(serde_json::json!({"status": "sent", "interactive": true})).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response(),
        };
    }

    // Resolve the session to find the provider and provider_session_id
    let store = match zag_agent::session::SessionStore::load(None) {
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

    let entry = match store.find_by_any_id(&id) {
        Some(e) => e.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", id),
                }),
            )
                .into_response();
        }
    };

    let provider_session_id = entry
        .provider_session_id
        .as_deref()
        .unwrap_or(&id)
        .to_string();

    let model = if entry.model.is_empty() {
        None
    } else {
        Some(entry.model.clone())
    };

    // Create agent and send message
    let agent_result =
        zag_agent::factory::AgentFactory::create(&entry.provider, None, model, None, false, vec![]);

    match agent_result {
        Ok(agent) => {
            match agent
                .run_resume_with_prompt(&provider_session_id, &req.message)
                .await
            {
                Ok(Some(output)) => {
                    Json(serde_json::to_value(&output).unwrap_or_default()).into_response()
                }
                Ok(None) => {
                    Json(serde_json::json!({"status": "sent", "output": null})).into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
