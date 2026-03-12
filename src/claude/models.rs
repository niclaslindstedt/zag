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

/// A user message containing tool results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub role: String,
    pub content: Vec<ToolResultBlock>,
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
}

/// A tool result block in a user message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
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
impl From<ClaudeOutput> for AgentOutput {
    fn from(claude_output: ClaudeOutput) -> Self {
        let mut session_id = String::from("unknown");
        let mut result = None;
        let mut is_error = false;
        let mut total_cost_usd = None;
        let mut usage = None;
        let mut events = Vec::new();

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
                    ..
                } => {
                    session_id = sid;

                    // Convert content blocks
                    let content: Vec<UnifiedContentBlock> = message
                        .content
                        .into_iter()
                        .map(|block| match block {
                            ContentBlock::Text { text } => UnifiedContentBlock::Text { text },
                            ContentBlock::ToolUse { id, name, input } => {
                                UnifiedContentBlock::ToolUse { id, name, input }
                            }
                        })
                        .collect();

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

                    events.push(UnifiedEvent::AssistantMessage {
                        content,
                        usage: msg_usage,
                    });
                }

                ClaudeEvent::User {
                    message,
                    tool_use_result,
                    session_id: sid,
                    ..
                } => {
                    session_id = sid;

                    // Convert tool results to tool execution events
                    for result_block in message.content {
                        // Try to find the corresponding tool use from previous assistant messages
                        let tool_name = find_tool_name(&events, &result_block.tool_use_id)
                            .unwrap_or_else(|| "unknown".to_string());

                        let tool_result = ToolResult {
                            success: !result_block.is_error,
                            output: if !result_block.is_error {
                                Some(result_block.content.clone())
                            } else {
                                None
                            },
                            error: if result_block.is_error {
                                Some(result_block.content.clone())
                            } else {
                                None
                            },
                            data: tool_use_result.clone(),
                        };

                        events.push(UnifiedEvent::ToolExecution {
                            tool_name,
                            tool_id: result_block.tool_use_id,
                            input: serde_json::Value::Null,
                            result: tool_result,
                        });
                    }
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
                    result = Some(res.clone());
                    total_cost_usd = Some(cost);

                    // Convert usage
                    usage = Some(UnifiedUsage {
                        input_tokens: u.input_tokens,
                        output_tokens: u.output_tokens,
                        cache_read_tokens: Some(u.cache_read_input_tokens),
                        cache_creation_tokens: Some(u.cache_creation_input_tokens),
                        web_search_requests: u
                            .server_tool_use
                            .as_ref()
                            .map(|s| s.web_search_requests),
                        web_fetch_requests: u
                            .server_tool_use
                            .as_ref()
                            .map(|s| s.web_fetch_requests),
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

                    // Add final result event
                    events.push(UnifiedEvent::Result {
                        success: !err,
                        message: Some(res),
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
            total_cost_usd,
            usage,
        }
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_response() {
        let json = r#"[
            {
                "type": "system",
                "subtype": "init",
                "session_id": "test123",
                "model": "claude-sonnet-4-5",
                "tools": ["Bash", "Read"],
                "uuid": "uuid1"
            },
            {
                "type": "assistant",
                "message": {
                    "model": "claude-sonnet-4-5",
                    "id": "msg1",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "Hello world"}
                    ],
                    "stop_reason": null,
                    "stop_sequence": null,
                    "usage": {
                        "input_tokens": 10,
                        "output_tokens": 5
                    }
                },
                "parent_tool_use_id": null,
                "session_id": "test123",
                "uuid": "uuid2"
            },
            {
                "type": "result",
                "subtype": "success",
                "is_error": false,
                "duration_ms": 1000,
                "duration_api_ms": 950,
                "num_turns": 1,
                "result": "Hello world",
                "session_id": "test123",
                "total_cost_usd": 0.001,
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5
                },
                "permission_denials": [],
                "uuid": "uuid3"
            }
        ]"#;

        let claude_output: ClaudeOutput = serde_json::from_str(json).expect("Failed to parse");
        let agent_output: AgentOutput = claude_output.into();

        assert_eq!(agent_output.agent, "claude");
        assert_eq!(agent_output.session_id, "test123");
        assert_eq!(agent_output.result, Some("Hello world".to_string()));
        assert!(!agent_output.is_error);
        assert_eq!(agent_output.total_cost_usd, Some(0.001));
    }
}
