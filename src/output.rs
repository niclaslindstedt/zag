/// Unified output structures for all agents.
///
/// This module provides a common interface for processing output from different
/// AI coding agents (Claude, Codex, Gemini, Copilot). By normalizing outputs into
/// a unified format, we can provide consistent logging, debugging, and observability
/// across all agents.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A unified event stream output from an agent session.
///
/// This represents the complete output from an agent execution, containing
/// all events that occurred during the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// The agent that produced this output
    pub agent: String,

    /// Unique session identifier
    pub session_id: String,

    /// Events that occurred during the session
    pub events: Vec<Event>,

    /// Final result text (if any)
    pub result: Option<String>,

    /// Whether the session ended in an error
    pub is_error: bool,

    /// Total cost in USD (if available)
    pub total_cost_usd: Option<f64>,

    /// Aggregated usage statistics
    pub usage: Option<Usage>,
}

/// A single event in an agent session.
///
/// Events represent discrete steps in the conversation flow, such as
/// initialization, messages, tool calls, and results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Session initialization event
    Init {
        model: String,
        tools: Vec<String>,
        working_directory: Option<String>,
        metadata: HashMap<String, serde_json::Value>,
    },

    /// Message from the assistant
    AssistantMessage {
        content: Vec<ContentBlock>,
        usage: Option<Usage>,
    },

    /// Tool execution event
    ToolExecution {
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
        result: ToolResult,
    },

    /// Final session result
    Result {
        success: bool,
        message: Option<String>,
        duration_ms: Option<u64>,
        num_turns: Option<u32>,
    },

    /// An error occurred
    Error {
        message: String,
        details: Option<serde_json::Value>,
    },

    /// Permission was requested
    PermissionRequest {
        tool_name: String,
        description: String,
        granted: bool,
    },
}

/// A block of content in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content
    Text { text: String },

    /// A tool invocation
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Result from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool execution succeeded
    pub success: bool,

    /// Text output from the tool
    pub output: Option<String>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Structured result data (tool-specific)
    pub data: Option<serde_json::Value>,
}

/// Usage statistics for an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Total input tokens
    pub input_tokens: u64,

    /// Total output tokens
    pub output_tokens: u64,

    /// Tokens read from cache (if applicable)
    pub cache_read_tokens: Option<u64>,

    /// Tokens written to cache (if applicable)
    pub cache_creation_tokens: Option<u64>,

    /// Number of web search requests (if applicable)
    pub web_search_requests: Option<u32>,

    /// Number of web fetch requests (if applicable)
    pub web_fetch_requests: Option<u32>,
}

/// Log level for agent events.
///
/// Used to categorize events for filtering and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A log entry extracted from agent output.
///
/// This is a simplified view of events suitable for logging and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,

    /// Log message
    pub message: String,

    /// Optional structured data
    pub data: Option<serde_json::Value>,

    /// Timestamp (if available)
    pub timestamp: Option<String>,
}

impl AgentOutput {
    /// Extract log entries from the agent output.
    ///
    /// This converts events into a flat list of log entries suitable for
    /// display or filtering.
    pub fn to_log_entries(&self, min_level: LogLevel) -> Vec<LogEntry> {
        let mut entries = Vec::new();

        for event in &self.events {
            if let Some(entry) = event_to_log_entry(event) {
                if entry.level >= min_level {
                    entries.push(entry);
                }
            }
        }

        entries
    }

    /// Get the final result text.
    pub fn final_result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    /// Check if the session completed successfully.
    pub fn is_success(&self) -> bool {
        !self.is_error
    }

    /// Get all tool executions from the session.
    pub fn tool_executions(&self) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| matches!(e, Event::ToolExecution { .. }))
            .collect()
    }

    /// Get all errors from the session.
    pub fn errors(&self) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| matches!(e, Event::Error { .. }))
            .collect()
    }
}

/// Convert an event to a log entry.
fn event_to_log_entry(event: &Event) -> Option<LogEntry> {
    match event {
        Event::Init { model, .. } => Some(LogEntry {
            level: LogLevel::Info,
            message: format!("Initialized with model {}", model),
            data: None,
            timestamp: None,
        }),

        Event::AssistantMessage { content, .. } => {
            // Extract text from content blocks
            let texts: Vec<String> = content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect();

            if !texts.is_empty() {
                Some(LogEntry {
                    level: LogLevel::Debug,
                    message: texts.join("\n"),
                    data: None,
                    timestamp: None,
                })
            } else {
                None
            }
        }

        Event::ToolExecution {
            tool_name, result, ..
        } => {
            let level = if result.success {
                LogLevel::Debug
            } else {
                LogLevel::Warn
            };

            let message = if result.success {
                format!("Tool '{}' executed successfully", tool_name)
            } else {
                format!(
                    "Tool '{}' failed: {}",
                    tool_name,
                    result.error.as_deref().unwrap_or("unknown error")
                )
            };

            Some(LogEntry {
                level,
                message,
                data: result.data.clone(),
                timestamp: None,
            })
        }

        Event::Result {
            success, message, ..
        } => {
            let level = if *success {
                LogLevel::Info
            } else {
                LogLevel::Error
            };

            Some(LogEntry {
                level,
                message: message.clone().unwrap_or_else(|| {
                    if *success {
                        "Session completed".to_string()
                    } else {
                        "Session failed".to_string()
                    }
                }),
                data: None,
                timestamp: None,
            })
        }

        Event::Error { message, details } => Some(LogEntry {
            level: LogLevel::Error,
            message: message.clone(),
            data: details.clone(),
            timestamp: None,
        }),

        Event::PermissionRequest {
            tool_name, granted, ..
        } => {
            let level = if *granted {
                LogLevel::Debug
            } else {
                LogLevel::Warn
            };

            let message = if *granted {
                format!("Permission granted for tool '{}'", tool_name)
            } else {
                format!("Permission denied for tool '{}'", tool_name)
            };

            Some(LogEntry {
                level,
                message,
                data: None,
                timestamp: None,
            })
        }
    }
}

impl std::fmt::Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self.level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };

        write!(f, "[{}] {}", level_str, self.message)
    }
}
