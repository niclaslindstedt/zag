//! Router configuration for the zag server.

use axum::{Router, middleware, routing};
use tower_http::cors::{Any, CorsLayer};

use crate::auth::{ServerState, auth_middleware};
use crate::handlers::{health, processes, sessions};
use crate::ws;

/// Build the complete axum router with all API routes.
pub fn build_router(state: ServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    use crate::handlers::{capability, config, mcp, orchestration, skills};

    Router::new()
        // Health check (no auth)
        .route("/api/v1/health", routing::get(health::health))
        // Session management
        .route("/api/v1/sessions", routing::get(sessions::list))
        .route("/api/v1/sessions/spawn", routing::post(sessions::spawn))
        .route("/api/v1/sessions/collect", routing::post(sessions::collect))
        .route("/api/v1/sessions/wait", routing::post(sessions::wait))
        .route(
            "/api/v1/sessions/summary",
            routing::post(orchestration::summary),
        )
        .route(
            "/api/v1/sessions/retry",
            routing::post(orchestration::retry),
        )
        .route("/api/v1/sessions/pipe", routing::post(orchestration::pipe))
        .route(
            "/api/v1/sessions/broadcast",
            routing::post(orchestration::broadcast),
        )
        .route(
            "/api/v1/sessions/{id}",
            routing::get(sessions::show)
                .delete(sessions::delete)
                .patch(sessions::update),
        )
        .route(
            "/api/v1/sessions/{id}/status",
            routing::get(sessions::status),
        )
        .route(
            "/api/v1/sessions/{id}/events",
            routing::get(sessions::events),
        )
        .route(
            "/api/v1/sessions/{id}/output",
            routing::get(sessions::output),
        )
        .route(
            "/api/v1/sessions/{id}/cancel",
            routing::post(sessions::cancel),
        )
        .route(
            "/api/v1/sessions/{id}/input",
            routing::post(sessions::input),
        )
        .route(
            "/api/v1/sessions/{id}/log",
            routing::post(orchestration::log),
        )
        .route(
            "/api/v1/sessions/{id}/env",
            routing::get(orchestration::env),
        )
        // WebSocket endpoints
        .route(
            "/api/v1/sessions/{id}/stream",
            routing::get(ws::stream::stream),
        )
        .route("/api/v1/subscribe", routing::get(ws::subscribe::subscribe))
        // Search, GC, Review
        .route("/api/v1/search", routing::post(orchestration::search))
        .route("/api/v1/gc", routing::post(orchestration::gc))
        .route("/api/v1/review", routing::post(orchestration::review))
        // Config, Capability, Skills, MCP
        .route("/api/v1/config", routing::post(config::config))
        .route("/api/v1/capability", routing::get(capability::capability))
        .route("/api/v1/skills", routing::post(skills::skills))
        .route("/api/v1/mcp", routing::post(mcp::mcp))
        // Process management
        .route("/api/v1/processes", routing::get(processes::list))
        .route("/api/v1/processes/{id}", routing::get(processes::show))
        .route(
            "/api/v1/processes/{id}/stop",
            routing::post(processes::stop),
        )
        .route(
            "/api/v1/processes/{id}/kill",
            routing::post(processes::kill),
        )
        // Middleware
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(cors)
        .with_state(state)
}
