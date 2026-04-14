/// Claude-specific JSON output models.
///
/// These structures directly map to the JSON output format produced by the
/// Claude CLI when running with `--output json` (verbose mode). They can be
/// deserialized from JSON and then converted to the unified `AgentOutput` format.
///
/// See README.md in this directory for detailed documentation on the output format.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::output::{
    AgentOutput, ContentBlock as UnifiedContentBlock, Event as UnifiedEvent, ToolResult,
    Usage as UnifiedUsage,
};

/// The root structure: an array of events.
pub type ClaudeOutput = Vec<ClaudeEvent>;

/// A single event in Claude's output stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeEvent {
    /// System initialization event
    System {
        subtype: String,
        session_id: String,
        cwd: Option<String>,
        model: String,
        tools: Vec<String>,
        #[serde(default)]
        mcp_servers: Vec<serde_json::Value>,
        #[serde(rename = "permissionMode")]
        permission_mode: Option<String>,
        #[serde(default)]
        slash_commands: Vec<String>,
        #[serde(default)]
        agents: Vec<String>,
        #[serde(default)]
        skills: Vec<serde_json::Value>,
        #[serde(default)]
        plugins: Vec<Plugin>,
        uuid: String,
        #[serde(flatten)]
        extra: HashMap<String, serde_json::Value>,
    },

    /// Assistant message event
    Assistant {
        message: Message,
        parent_tool_use_id: Option<String>,
        session_id: String,
        uuid: String,
    },

    /// User message event (tool results)
    User {
        message: UserMessage,
        parent_tool_use_id: Option<String>,
        session_id: String,
        uuid: String,
        tool_use_result: Option<serde_json::Value>,
    },

    /// Final result event
    Result {
        subtype: String,
        is_error: bool,
        duration_ms: u64,
        duration_api_ms: u64,
        num_turns: u32,
        result: String,
        session_id: String,
        total_cost_usd: f64,
        usage: Usage,
        #[serde(default, rename = "modelUsage")]
        model_usage: HashMap<String, ModelUsage>,
        #[serde(default)]
        permission_denials: Vec<PermissionDenial>,
        uuid: String,
    },

    /// Unknown/unhandled event type (e.g., rate_limit_event) — silently ignored
    #[serde(other)]
    Other,
}

/// An assistant message from Claude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub model: String,
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
    pub context_management: Option<serde_json::Value>,
}

/// A user message containing tool results and other content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub role: String,
    pub content: Vec<UserContentBlock>,
}

/// A content block in an assistant message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },

    /// Tool invocation
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Thinking content (extended thinking)
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(flatten)]
        extra: HashMap<String, serde_json::Value>,
    },
}

/// A content block in a user message (tool results, text, or other types).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContentBlock {
    /// Tool result
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    /// Text content
    Text { text: String },

    /// Any other content type
    #[serde(other)]
    Other,
}

/// Usage statistics for a message or session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation: Option<CacheCreation>,
    #[serde(default)]
    pub server_tool_use: Option<ServerToolUse>,
    #[serde(default)]
    pub service_tier: Option<String>,
}

/// Cache creation details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCreation {
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u64,
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u64,
}

/// Server-side tool usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolUse {
    #[serde(default)]
    pub web_search_requests: u32,
    #[serde(default)]
    pub web_fetch_requests: u32,
}

/// Per-model usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,
    #[serde(default, rename = "cacheReadInputTokens")]
    pub cache_read_input_tokens: u64,
    #[serde(default, rename = "cacheCreationInputTokens")]
    pub cache_creation_input_tokens: u64,
    #[serde(default, rename = "webSearchRequests")]
    pub web_search_requests: u32,
    #[serde(rename = "costUSD")]
    pub cost_usd: f64,
    #[serde(default, rename = "contextWindow")]
    pub context_window: u64,
    #[serde(default, rename = "maxOutputTokens")]
    pub max_output_tokens: u64,
}

/// Information about a denied permission request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDenial {
    pub tool_name: String,
    pub tool_use_id: String,
    pub tool_input: serde_json::Value,
}

/// Plugin information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub path: String,
}

/// Convert Claude output to unified agent output.
pub fn claude_output_to_agent_output(claude_output: ClaudeOutput) -> AgentOutput {
    let mut session_id = String::from("unknown");
    let mut result = None;
    let mut is_error = false;
    let mut total_cost_usd = None;
    let mut usage = None;
    let mut events = Vec::new();
    let mut model_name: Option<String> = None;

    // Turn-boundary state for synthesizing Event::TurnComplete before each
    // Event::Result. Mirrors `ClaudeEventTranslator` in the streaming path
    // but is inlined here because the full-parse path also does its own
    // metadata extraction (session_id, total_cost_usd, ...) that doesn't
    // fit the translator's per-event shape.
    let mut pending_stop_reason: Option<String> = None;
    let mut pending_turn_usage: Option<UnifiedUsage> = None;
    let mut next_turn_index: u32 = 0;

    // Track text from the last assistant message for fallback when
    // Result.result is empty (e.g. when --json-schema is used, Claude Code
    // may put the content in the assistant message but leave the result
    // field blank).
    let mut last_assistant_text: Option<String> = None;

    for event in claude_output {
        match event {
            ClaudeEvent::System {
                session_id: sid,
                model,
                tools,
                cwd,
                mut extra,
                ..
            } => {
                session_id = sid;
                model_name = Some(model.clone());

                // Include all extra fields as metadata
                if let Some(cwd) = cwd {
                    extra.insert("cwd".to_string(), serde_json::json!(cwd));
                }

                events.push(UnifiedEvent::Init {
                    model,
                    tools,
                    working_directory: extra
                        .get("cwd")
                        .and_then(|v| v.as_str().map(|s| s.to_string())),
                    metadata: extra,
                });
            }

            ClaudeEvent::Assistant {
                message,
                session_id: sid,
                parent_tool_use_id,
                ..
            } => {
                session_id = sid;

                // Track the latest stop_reason for the current turn; the
                // final assistant message before a Result is the one whose
                // stop_reason explains why the turn ended.
                if let Some(reason) = &message.stop_reason {
                    pending_stop_reason = Some(reason.clone());
                }

                // Convert content blocks (skip thinking blocks)
                let content: Vec<UnifiedContentBlock> = message
                    .content
                    .into_iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(UnifiedContentBlock::Text { text }),
                        ContentBlock::ToolUse { id, name, input } => {
                            Some(UnifiedContentBlock::ToolUse { id, name, input })
                        }
                        ContentBlock::Thinking { .. } => None,
                    })
                    .collect();

                // Collect text blocks for fallback result extraction.
                let text_parts: Vec<&str> = content
                    .iter()
                    .filter_map(|b| match b {
                        UnifiedContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                if !text_parts.is_empty() {
                    last_assistant_text = Some(text_parts.join("\n"));
                }

                // Convert usage
                let msg_usage = Some(UnifiedUsage {
                    input_tokens: message.usage.input_tokens,
                    output_tokens: message.usage.output_tokens,
                    cache_read_tokens: Some(message.usage.cache_read_input_tokens),
                    cache_creation_tokens: Some(message.usage.cache_creation_input_tokens),
                    web_search_requests: message
                        .usage
                        .server_tool_use
                        .as_ref()
                        .map(|s| s.web_search_requests),
                    web_fetch_requests: message
                        .usage
                        .server_tool_use
                        .as_ref()
                        .map(|s| s.web_fetch_requests),
                });
                pending_turn_usage = msg_usage.clone();

                events.push(UnifiedEvent::AssistantMessage {
                    content,
                    usage: msg_usage,
                    parent_tool_use_id,
                });
            }

            ClaudeEvent::User {
                message,
                tool_use_result,
                session_id: sid,
                parent_tool_use_id,
                ..
            } => {
                session_id = sid;

                // Convert tool results to tool execution events (skip non-tool-result blocks)
                for block in message.content {
                    if let UserContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } = block
                    {
                        let tool_name = find_tool_name(&events, &tool_use_id)
                            .unwrap_or_else(|| "unknown".to_string());

                        let tool_result = ToolResult {
                            success: !is_error,
                            output: if !is_error {
                                Some(content.clone())
                            } else {
                                None
                            },
                            error: if is_error {
                                Some(content.clone())
                            } else {
                                None
                            },
                            data: tool_use_result.clone(),
                        };

                        events.push(UnifiedEvent::ToolExecution {
                            tool_name,
                            tool_id: tool_use_id,
                            input: serde_json::Value::Null,
                            result: tool_result,
                            parent_tool_use_id: parent_tool_use_id.clone(),
                        });
                    }
                }
            }

            ClaudeEvent::Other => {
                log::debug!("Skipping unknown Claude event type during output conversion");
            }

            ClaudeEvent::Result {
                is_error: err,
                result: res,
                total_cost_usd: cost,
                usage: u,
                duration_ms,
                num_turns,
                permission_denials,
                session_id: sid,
                subtype: _,
                ..
            } => {
                session_id = sid;
                is_error = err;

                // When Result.result is empty, fall back to the last assistant
                // message text.  Claude Code sometimes puts the actual content
                // (especially --json-schema output) in the assistant message
                // while leaving the result field blank.
                let effective_result = if res.is_empty() {
                    if let Some(ref fallback) = last_assistant_text {
                        log::debug!(
                            "Result.result is empty; using last assistant text ({} bytes)",
                            fallback.len()
                        );
                        fallback.clone()
                    } else {
                        res.clone()
                    }
                } else {
                    res.clone()
                };

                result = Some(effective_result.clone());
                total_cost_usd = Some(cost);

                // Convert usage
                usage = Some(UnifiedUsage {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    cache_read_tokens: Some(u.cache_read_input_tokens),
                    cache_creation_tokens: Some(u.cache_creation_input_tokens),
                    web_search_requests: u.server_tool_use.as_ref().map(|s| s.web_search_requests),
                    web_fetch_requests: u.server_tool_use.as_ref().map(|s| s.web_fetch_requests),
                });

                // Add permission denial events
                for denial in permission_denials {
                    events.push(UnifiedEvent::PermissionRequest {
                        tool_name: denial.tool_name,
                        description: format!(
                            "Permission denied for tool input: {}",
                            serde_json::to_string(&denial.tool_input).unwrap_or_default()
                        ),
                        granted: false,
                    });
                }

                // Emit TurnComplete immediately before the per-turn Result.
                events.push(UnifiedEvent::TurnComplete {
                    stop_reason: pending_stop_reason.take(),
                    turn_index: next_turn_index,
                    usage: pending_turn_usage.take(),
                });
                next_turn_index = next_turn_index.saturating_add(1);

                // Add final result event
                events.push(UnifiedEvent::Result {
                    success: !err,
                    message: Some(effective_result),
                    duration_ms: Some(duration_ms),
                    num_turns: Some(num_turns),
                });
            }
        }
    }

    AgentOutput {
        agent: "claude".to_string(),
        session_id,
        events,
        result,
        is_error,
        exit_code: None,
        error_message: None,
        total_cost_usd,
        usage,
        model: model_name,
        provider: Some("claude".to_string()),
    }
}

/// Find the tool name for a given tool_use_id by searching previous events.
fn find_tool_name(events: &[UnifiedEvent], tool_use_id: &str) -> Option<String> {
    for event in events.iter().rev() {
        if let UnifiedEvent::AssistantMessage { content, .. } = event {
            for block in content {
                if let UnifiedContentBlock::ToolUse { id, name, .. } = block
                    && id == tool_use_id
                {
                    return Some(name.clone());
                }
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
