/// Unified output structures for all agents.
///
/// This module provides a common interface for processing output from different
/// AI coding agents (Claude, Codex, Gemini, Copilot). By normalizing outputs into
/// a unified format, we can provide consistent logging, debugging, and observability
/// across all agents.
use log::debug;
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
    /// Create a minimal AgentOutput from captured text.
    ///
    /// Used by non-Claude agents when `capture_output` is enabled (e.g., for auto-selection).
    pub fn from_text(agent: &str, text: &str) -> Self {
        debug!(
            "Creating AgentOutput from text: agent={}, len={}",
            agent,
            text.len()
        );
        Self {
            agent: agent.to_string(),
            session_id: String::new(),
            events: vec![Event::Result {
                success: true,
                message: Some(text.to_string()),
                duration_ms: None,
                num_turns: None,
            }],
            result: Some(text.to_string()),
            is_error: false,
            total_cost_usd: None,
            usage: None,
        }
    }

    /// Extract log entries from the agent output.
    ///
    /// This converts events into a flat list of log entries suitable for
    /// display or filtering.
    pub fn to_log_entries(&self, min_level: LogLevel) -> Vec<LogEntry> {
        debug!(
            "Extracting log entries from {} events (min_level={:?})",
            self.events.len(),
            min_level
        );
        let mut entries = Vec::new();

        for event in &self.events {
            if let Some(entry) = event_to_log_entry(event)
                && entry.level >= min_level
            {
                entries.push(entry);
            }
        }

        entries
    }

    /// Get the final result text.
    pub fn final_result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    /// Check if the session completed successfully.
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        !self.is_error
    }

    /// Get all tool executions from the session.
    #[allow(dead_code)]
    pub fn tool_executions(&self) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| matches!(e, Event::ToolExecution { .. }))
            .collect()
    }

    /// Get all errors from the session.
    #[allow(dead_code)]
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

/// Get a consistent color for a tool ID using round-robin color selection.
fn get_tool_id_color(tool_id: &str) -> &'static str {
    // 10 distinct colors for tool IDs
    const TOOL_COLORS: [&str; 10] = [
        "\x1b[38;5;33m",  // Blue
        "\x1b[38;5;35m",  // Green
        "\x1b[38;5;141m", // Purple
        "\x1b[38;5;208m", // Orange
        "\x1b[38;5;213m", // Pink
        "\x1b[38;5;51m",  // Cyan
        "\x1b[38;5;226m", // Yellow
        "\x1b[38;5;205m", // Magenta
        "\x1b[38;5;87m",  // Aqua
        "\x1b[38;5;215m", // Peach
    ];

    // Hash the tool_id to get a consistent color
    let hash: u32 = tool_id.bytes().map(|b| b as u32).sum();
    let index = (hash as usize) % TOOL_COLORS.len();
    TOOL_COLORS[index]
}

/// Format a single event as beautiful text output.
///
/// This can be used to stream events in real-time with nice formatting.
pub fn format_event_as_text(event: &Event) -> Option<String> {
    const INDENT: &str = "    ";
    const INDENT_RESULT: &str = "      "; // 6 spaces for tool result continuation
    const RECORD_ICON: &str = "⏺";
    const ARROW_ICON: &str = "←";
    const ORANGE: &str = "\x1b[38;5;208m";
    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const DIM: &str = "\x1b[38;5;240m"; // Gray color for better visibility than dim
    const RESET: &str = "\x1b[0m";

    match event {
        Event::Init { model, .. } => {
            Some(format!("\x1b[32m✓\x1b[0m Initialized with model {}", model))
        }

        Event::AssistantMessage { content, .. } => {
            let formatted: Vec<String> = content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => {
                        // Orange text with record icon, indented
                        // Handle multi-line text - first line with icon, rest indented 6 spaces
                        let lines: Vec<&str> = text.lines().collect();
                        if lines.is_empty() {
                            None
                        } else {
                            let mut formatted_lines = Vec::new();
                            for (i, line) in lines.iter().enumerate() {
                                if i == 0 {
                                    // First line with record icon
                                    formatted_lines.push(format!(
                                        "{}{}{} {}{}",
                                        INDENT, ORANGE, RECORD_ICON, line, RESET
                                    ));
                                } else {
                                    // Subsequent lines, indented 6 spaces (still orange)
                                    formatted_lines.push(format!(
                                        "{}{}{}{}",
                                        INDENT_RESULT, ORANGE, line, RESET
                                    ));
                                }
                            }
                            Some(formatted_lines.join("\n"))
                        }
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        // Tool call with colored id (last 4 chars)
                        let id_suffix = &id[id.len().saturating_sub(4)..];
                        let id_color = get_tool_id_color(id_suffix);
                        const BLUE: &str = "\x1b[34m";

                        // Special formatting for Bash tool
                        if name == "Bash"
                            && let serde_json::Value::Object(obj) = input
                        {
                            let description = obj
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Run command");
                            let command = obj.get("command").and_then(|v| v.as_str()).unwrap_or("");

                            return Some(format!(
                                "{}{}{} {}{} {}[{}]{}\n{}{}└── {}{}",
                                INDENT,
                                BLUE,
                                RECORD_ICON,
                                description,
                                RESET,
                                id_color,
                                id_suffix,
                                RESET,
                                INDENT_RESULT,
                                DIM,
                                command,
                                RESET
                            ));
                        }

                        // Format input parameters for non-Bash tools
                        let input_str = if let serde_json::Value::Object(obj) = input {
                            if obj.is_empty() {
                                String::new()
                            } else {
                                // Format the parameters as key=value pairs
                                let params: Vec<String> = obj
                                    .iter()
                                    .map(|(key, value)| {
                                        let value_str = match value {
                                            serde_json::Value::String(s) => {
                                                // Truncate long strings
                                                if s.len() > 60 {
                                                    format!("\"{}...\"", &s[..57])
                                                } else {
                                                    format!("\"{}\"", s)
                                                }
                                            }
                                            serde_json::Value::Number(n) => n.to_string(),
                                            serde_json::Value::Bool(b) => b.to_string(),
                                            serde_json::Value::Null => "null".to_string(),
                                            _ => "...".to_string(),
                                        };
                                        format!("{}={}", key, value_str)
                                    })
                                    .collect();
                                params.join(", ")
                            }
                        } else {
                            "...".to_string()
                        };

                        Some(format!(
                            "{}{}{} {}({}) {}[{}]{}",
                            INDENT, BLUE, RECORD_ICON, name, input_str, id_color, id_suffix, RESET
                        ))
                    }
                })
                .collect();

            if !formatted.is_empty() {
                // Add blank line after
                Some(format!("{}\n", formatted.join("\n")))
            } else {
                None
            }
        }

        Event::ToolExecution {
            tool_id, result, ..
        } => {
            let id_suffix = &tool_id[tool_id.len().saturating_sub(4)..];
            let id_color = get_tool_id_color(id_suffix);
            let (icon_color, status_text) = if result.success {
                (GREEN, "success")
            } else {
                (RED, "failed")
            };

            // Get full result text (all lines)
            let result_text = if result.success {
                result.output.as_deref().unwrap_or(status_text)
            } else {
                result.error.as_deref().unwrap_or(status_text)
            };

            // Split into lines and format each one
            let mut lines: Vec<&str> = result_text.lines().collect();
            if lines.is_empty() {
                lines.push(status_text);
            }

            let mut formatted_lines = Vec::new();

            // First line: arrow icon with tool ID
            formatted_lines.push(format!(
                "{}{}{}{} {}[{}]{}",
                INDENT, icon_color, ARROW_ICON, RESET, id_color, id_suffix, RESET
            ));

            // All result lines indented at 6 spaces
            for line in lines.iter() {
                formatted_lines.push(format!("{}{}{}{}", INDENT_RESULT, DIM, line, RESET));
            }

            // Add blank line after
            Some(format!("{}\n", formatted_lines.join("\n")))
        }

        Event::Result { .. } => {
            // Don't output the final result since it's already been streamed
            None
        }

        Event::Error { message, .. } => Some(format!("\x1b[31mError:\x1b[0m {}", message)),

        Event::PermissionRequest {
            tool_name, granted, ..
        } => {
            if *granted {
                Some(format!(
                    "\x1b[32m✓\x1b[0m Permission granted for tool '{}'",
                    tool_name
                ))
            } else {
                Some(format!(
                    "\x1b[33m!\x1b[0m Permission denied for tool '{}'",
                    tool_name
                ))
            }
        }
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
