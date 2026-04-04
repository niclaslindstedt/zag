//! API request and response types for the zag server.

use serde::{Deserialize, Serialize};

/// Request body for POST /api/v1/sessions/spawn
#[derive(Debug, Deserialize)]
pub struct SpawnRequest {
    pub prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: Option<bool>,
    pub system_prompt: Option<String>,
    pub add_dirs: Option<Vec<String>>,
    pub size: Option<String>,
    pub max_turns: Option<u32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub inject_context: Option<bool>,
}

/// Response for POST /api/v1/sessions/spawn
#[derive(Debug, Serialize)]
pub struct SpawnResponse {
    pub session_id: String,
    pub pid: u32,
    pub log_path: String,
}

/// Request body for POST /api/v1/sessions/:id/input
#[derive(Debug, Deserialize)]
pub struct InputRequest {
    pub message: String,
}

/// Request body for POST /api/v1/sessions/:id/cancel
#[derive(Debug, Deserialize)]
pub struct CancelRequest {
    pub reason: Option<String>,
}

/// Request body for POST /api/v1/sessions/collect
#[derive(Debug, Deserialize)]
pub struct CollectRequest {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
}

/// Request body for POST /api/v1/sessions/wait
#[derive(Debug, Deserialize)]
pub struct WaitRequest {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub timeout: Option<String>,
    pub any: Option<bool>,
}

/// Query parameters for GET /api/v1/sessions
#[derive(Debug, Deserialize)]
pub struct SessionListQuery {
    pub tag: Option<String>,
    pub provider: Option<String>,
    pub limit: Option<usize>,
    pub global: Option<bool>,
}

/// Query parameters for GET /api/v1/sessions/:id/events
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub last: Option<usize>,
    pub after_seq: Option<u64>,
    pub before_seq: Option<u64>,
}

/// Query parameters for WebSocket subscribe
#[derive(Debug, Deserialize)]
pub struct SubscribeQuery {
    pub tag: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
}

/// Query parameters for WebSocket stream
#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    pub filter: Option<String>,
}

/// Query parameters for GET /api/v1/processes
#[derive(Debug, Deserialize)]
pub struct ProcessListQuery {
    pub running: Option<bool>,
    pub limit: Option<usize>,
    pub provider: Option<String>,
}

/// Standard error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
