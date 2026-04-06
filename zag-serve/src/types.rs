//! API request and response types for the zag server.

use serde::{Deserialize, Serialize};

/// Request body for POST /api/v1/sessions/spawn
#[derive(Debug, Deserialize)]
pub struct SpawnRequest {
    pub prompt: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: Option<bool>,
    pub system_prompt: Option<String>,
    pub add_dirs: Option<Vec<String>>,
    pub size: Option<String>,
    pub max_turns: Option<u32>,
    pub timeout: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub inject_context: Option<bool>,
    pub interactive: Option<bool>,
}

/// Response for POST /api/v1/sessions/spawn
#[derive(Debug, Serialize)]
pub struct SpawnResponse {
    pub session_id: String,
    pub pid: u32,
    pub log_path: String,
    pub interactive: bool,
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

/// Request body for POST /api/v1/sessions/summary
#[derive(Debug, Deserialize)]
pub struct SummaryRequest {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub stats: Option<bool>,
}

/// Request body for POST /api/v1/sessions/retry
#[derive(Debug, Deserialize)]
pub struct RetryRequest {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub failed: Option<bool>,
    pub model: Option<String>,
}

/// Request body for POST /api/v1/gc
#[derive(Debug, Deserialize)]
pub struct GcRequest {
    pub force: Option<bool>,
    pub older_than: Option<String>,
    pub keep_logs: Option<bool>,
}

/// Request body for POST /api/v1/sessions/{id}/log
#[derive(Debug, Deserialize)]
pub struct LogRequest {
    pub message: String,
    pub level: Option<String>,
    pub data: Option<String>,
}

/// Request body for POST /api/v1/sessions/pipe
#[derive(Debug, Deserialize)]
pub struct PipeRequest {
    pub session_ids: Vec<String>,
    pub tag: Option<String>,
    pub prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: Option<bool>,
    pub system_prompt: Option<String>,
    pub add_dirs: Option<Vec<String>>,
    pub size: Option<String>,
    pub max_turns: Option<u32>,
}

/// Request body for POST /api/v1/sessions/broadcast
#[derive(Debug, Deserialize)]
pub struct BroadcastRequest {
    pub message: String,
    pub tag: Option<String>,
    pub global: Option<bool>,
    pub raw: Option<bool>,
}

/// Request body for POST /api/v1/review
#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub uncommitted: Option<bool>,
    pub base: Option<String>,
    pub commit: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub root: Option<String>,
    pub auto_approve: Option<bool>,
    pub add_dirs: Option<Vec<String>>,
}

/// Request body for POST /api/v1/search
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub regex: Option<bool>,
    pub case_sensitive: Option<bool>,
    pub provider: Option<String>,
    pub role: Option<String>,
    pub tool: Option<String>,
    pub tool_kind: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub session: Option<String>,
    pub tag: Option<String>,
    pub global: Option<bool>,
    pub count: Option<bool>,
    pub limit: Option<usize>,
}

/// Request body for POST /api/v1/config
#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub args: Vec<String>,
    pub root: Option<String>,
}

/// Request body for POST /api/v1/skills
#[derive(Debug, Deserialize)]
pub struct SkillsRequest {
    pub command: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub provider: Option<String>,
    pub from: Option<String>,
}

/// Request body for POST /api/v1/mcp
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub command: String,
    pub name: Option<String>,
    pub transport: Option<String>,
    #[serde(rename = "command_str")]
    pub server_command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub env: Option<Vec<String>>,
    pub description: Option<String>,
    pub global: Option<bool>,
    pub root: Option<String>,
    pub from: Option<String>,
}

/// Query parameters for GET /api/v1/capability
#[derive(Debug, Deserialize)]
pub struct CapabilityQuery {
    pub provider: Option<String>,
    pub format: Option<String>,
    pub pretty: Option<bool>,
}

/// Request body for PATCH /api/v1/sessions/{id}
#[derive(Debug, Deserialize)]
pub struct SessionUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub clear_tags: Option<bool>,
}

/// Query parameters for GET /api/v1/sessions/{id}/env
#[derive(Debug, Deserialize)]
pub struct EnvQuery {
    pub shell: Option<bool>,
}

/// Request body for POST /api/v1/users/add
#[derive(Debug, Deserialize)]
pub struct UserAddRequest {
    pub username: String,
    pub password: String,
    pub home_dir: String,
}

/// Request body for POST /api/v1/users/remove
#[derive(Debug, Deserialize)]
pub struct UserRemoveRequest {
    pub username: String,
}

/// Request body for POST /api/v1/users/passwd
#[derive(Debug, Deserialize)]
pub struct UserPasswdRequest {
    pub username: String,
    pub password: String,
}

/// Response for user management endpoints
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub message: String,
}

/// A single user entry in list responses (no password hash).
#[derive(Debug, Serialize)]
pub struct UserListEntry {
    pub username: String,
    pub home_dir: String,
    pub enabled: bool,
    pub created_at: String,
}

/// Request body for POST /api/v1/login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response for POST /api/v1/login
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub home_dir: String,
}

/// Response for POST /api/v1/logout
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}
