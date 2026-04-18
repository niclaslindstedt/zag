//! Orchestration HTTP handlers for commands backed by zag-orch functions.

use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
};

use crate::types::*;

/// POST /api/v1/sessions/summary
pub async fn summary(Json(req): Json<SummaryRequest>) -> impl IntoResponse {
    let params = zag_orch::summary::SummaryParams {
        session_ids: req.session_ids,
        tag: req.tag,
        stats: req.stats.unwrap_or(false),
        json: true,
        root: None,
    };

    match zag_orch::summary::summarize_sessions(&params) {
        Ok(summaries) => Json(serde_json::to_value(&summaries).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/retry
pub async fn retry(Json(req): Json<RetryRequest>) -> impl IntoResponse {
    let params = zag_orch::retry::RetryParams {
        session_ids: req.session_ids,
        tag: req.tag,
        failed: req.failed.unwrap_or(false),
        model: req.model,
        json: true,
        root: None,
    };

    match zag_orch::retry::retry_sessions(&params) {
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

/// POST /api/v1/gc
pub async fn gc(Json(req): Json<GcRequest>) -> impl IntoResponse {
    let params = zag_orch::gc::GcParams {
        force: req.force.unwrap_or(false),
        older_than: req.older_than.unwrap_or_else(|| "7d".to_string()),
        keep_logs: req.keep_logs.unwrap_or(false),
        json: true,
        root: None,
    };

    match zag_orch::gc::gc_collect(&params) {
        Ok(report) => Json(serde_json::to_value(&report).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/{id}/log
pub async fn log(Path(id): Path<String>, Json(req): Json<LogRequest>) -> impl IntoResponse {
    let params = zag_orch::log_cmd::LogParams {
        message: req.message,
        session: Some(id),
        level: req.level.unwrap_or_else(|| "info".to_string()),
        data: req.data,
        root: None,
    };

    match zag_orch::log_cmd::run_log(params) {
        Ok(()) => Json(serde_json::json!({"status": "logged"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/sessions/{id}/env
pub async fn env(Path(id): Path<String>, Query(query): Query<EnvQuery>) -> impl IntoResponse {
    let _ = query; // shell formatting is a client concern
    match zag_orch::env::get_env_vars(Some(&id), None) {
        Ok(vars) => {
            let map: serde_json::Map<String, serde_json::Value> = vars
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            Json(serde_json::Value::Object(map)).into_response()
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

/// POST /api/v1/search
pub async fn search(Json(req): Json<SearchRequest>) -> impl IntoResponse {
    let tool_kind = req.tool_kind.as_deref().and_then(|k| match k {
        "shell" => Some(zag_agent::session_log::ToolKind::Shell),
        "file_read" | "read" => Some(zag_agent::session_log::ToolKind::FileRead),
        "file_write" | "write" => Some(zag_agent::session_log::ToolKind::FileWrite),
        "file_edit" | "edit" => Some(zag_agent::session_log::ToolKind::FileEdit),
        "search" => Some(zag_agent::session_log::ToolKind::Search),
        "sub_agent" => Some(zag_agent::session_log::ToolKind::SubAgent),
        "web" => Some(zag_agent::session_log::ToolKind::Web),
        "notebook" => Some(zag_agent::session_log::ToolKind::Notebook),
        _ => None,
    });

    let args = zag_orch::search::SearchCommandArgs {
        query: req.query,
        use_regex: req.regex.unwrap_or(false),
        case_sensitive: req.case_sensitive.unwrap_or(false),
        provider: req.provider,
        role: req.role,
        tool: req.tool,
        tool_kind,
        from: req.from,
        to: req.to,
        session: req.session,
        tag: req.tag,
        global: req.global.unwrap_or(false),
        json: true,
        count: req.count.unwrap_or(false),
        limit: req.limit,
        root: None,
    };

    // run_search_command prints to stdout; we need to capture it
    // For now, call it and let it print — the proxy will get the response as JSON
    match zag_orch::search::run_search_command(args, true) {
        Ok(()) => Json(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/pipe
pub async fn pipe(Json(req): Json<PipeRequest>) -> impl IntoResponse {
    let params = zag_orch::pipe::PipeParams {
        session_ids: req.session_ids,
        tag: req.tag,
        prompt: req.prompt,
        provider: req.provider,
        model: req.model,
        root: req.root,
        auto_approve: req.auto_approve.unwrap_or(false),
        system_prompt: req.system_prompt,
        add_dirs: req.add_dirs.unwrap_or_default(),
        size: req.size,
        max_turns: req.max_turns,
        output: Some("json".to_string()),
        json: true,
        quiet: true,
        metadata: zag_orch::types::SessionMetadata {
            name: req.name,
            description: req.description,
            tags: req.tags.unwrap_or_default(),
        },
        timeout: req.timeout,
        env_vars: req.env_vars.unwrap_or_default(),
        files: req.files.unwrap_or_default(),
        worktree: req.worktree,
        sandbox: req.sandbox,
        context: req.context,
        mcp_config: req.mcp_config,
    };

    match zag_orch::pipe::pipe_sessions(&params).await {
        Ok(output) => Json(serde_json::to_value(&output).unwrap_or_default()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/sessions/broadcast
pub async fn broadcast(Json(req): Json<BroadcastRequest>) -> impl IntoResponse {
    let store = if req.global.unwrap_or(false) {
        zag_agent::session::SessionStore::load_all().unwrap_or_default()
    } else {
        zag_agent::session::SessionStore::load(None).unwrap_or_default()
    };

    let session_ids: Vec<String> = if let Some(ref tag) = req.tag {
        let matches = store.find_by_tag(tag);
        if matches.is_empty() {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("No sessions found with tag '{tag}'"),
                }),
            )
                .into_response();
        }
        matches.iter().map(|e| e.session_id.clone()).collect()
    } else {
        store
            .sessions
            .iter()
            .map(|e| e.session_id.clone())
            .collect()
    };

    let mut results = Vec::new();
    for id in &session_ids {
        let entry = match store.find_by_any_id(id) {
            Some(e) => e.clone(),
            None => continue,
        };

        let provider_session_id = entry
            .provider_session_id
            .as_deref()
            .unwrap_or(id)
            .to_string();

        let model = if entry.model.is_empty() {
            None
        } else {
            Some(entry.model.clone())
        };

        let agent_result = zag_agent::factory::AgentFactory::create(
            &entry.provider,
            None,
            model,
            None,
            false,
            vec![],
        );

        match agent_result {
            Ok(agent) => {
                match agent
                    .run_resume_with_prompt(&provider_session_id, &req.message)
                    .await
                {
                    Ok(output) => {
                        results.push(serde_json::json!({
                            "session_id": id,
                            "status": "sent",
                            "output": output,
                        }));
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "session_id": id,
                            "status": "error",
                            "error": e.to_string(),
                        }));
                    }
                }
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "session_id": id,
                    "status": "error",
                    "error": e.to_string(),
                }));
            }
        }
    }

    Json(serde_json::to_value(&results).unwrap_or_default()).into_response()
}

/// POST /api/v1/review
pub async fn review(Json(req): Json<ReviewRequest>) -> impl IntoResponse {
    let uncommitted = req.uncommitted.unwrap_or(false);
    let auto_approve = req.auto_approve.unwrap_or(false);

    if !uncommitted && req.base.is_none() && req.commit.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Review requires at least one of: uncommitted, base, commit".to_string(),
            }),
        )
            .into_response();
    }

    let mut agent = match zag_agent::factory::AgentFactory::create(
        "codex",
        None,
        req.model,
        req.root.clone(),
        auto_approve,
        req.add_dirs.unwrap_or_default(),
    ) {
        Ok(a) => a,
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

    let codex = match agent
        .as_any_mut()
        .downcast_mut::<zag_agent::providers::codex::Codex>()
    {
        Some(c) => c,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get Codex agent for review".to_string(),
                }),
            )
                .into_response();
        }
    };

    match codex
        .review(
            uncommitted,
            req.base.as_deref(),
            req.commit.as_deref(),
            req.title.as_deref(),
        )
        .await
    {
        Ok(()) => Json(serde_json::json!({"status": "completed"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}
