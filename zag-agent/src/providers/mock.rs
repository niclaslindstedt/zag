//! Mock agent for integration testing.
//!
//! This module provides a configurable mock agent that implements the full
//! `Agent` trait without requiring any external CLI binary. It is only
//! available in test builds (`#[cfg(test)]`).
//!
//! # Examples
//!
//! ```rust
//! use zag_agent::providers::mock::{MockAgent, MockResponse};
//!
//! let agent = MockAgent::builder()
//!     .respond_with_text("hello world")
//!     .respond_with_error("something failed")
//!     .build();
//! ```

use crate::agent::{Agent, ModelSize};
use crate::output::{AgentOutput, ContentBlock, Event, ToolResult, Usage};
use crate::sandbox::SandboxConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const DEFAULT_MODEL: &str = "mock-default";

pub const AVAILABLE_MODELS: &[&str] = &["mock-default", "mock-small", "mock-medium", "mock-large"];

/// A configurable response that the mock agent returns from `run()`.
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// The result text to include in the output.
    pub result: Option<String>,
    /// Events to include in the output.
    pub events: Vec<Event>,
    /// Session ID for the output.
    pub session_id: String,
    /// Whether this response represents an error.
    pub is_error: bool,
    /// Total cost in USD.
    pub total_cost_usd: Option<f64>,
    /// Usage statistics.
    pub usage: Option<Usage>,
}

impl MockResponse {
    /// Create a simple text response.
    pub fn text(text: &str) -> Self {
        Self {
            result: Some(text.to_string()),
            events: vec![Event::Result {
                success: true,
                message: Some(text.to_string()),
                duration_ms: Some(100),
                num_turns: Some(1),
            }],
            session_id: uuid::Uuid::new_v4().to_string(),
            is_error: false,
            total_cost_usd: None,
            usage: None,
        }
    }

    /// Create an error response.
    pub fn error(message: &str) -> Self {
        Self {
            result: Some(message.to_string()),
            events: vec![Event::Error {
                message: message.to_string(),
                details: None,
            }],
            session_id: uuid::Uuid::new_v4().to_string(),
            is_error: true,
            total_cost_usd: None,
            usage: None,
        }
    }

    /// Create a response with custom events.
    pub fn with_events(events: Vec<Event>) -> Self {
        let result = events.iter().find_map(|e| {
            if let Event::Result { message, .. } = e {
                message.clone()
            } else {
                None
            }
        });
        Self {
            result,
            events,
            session_id: uuid::Uuid::new_v4().to_string(),
            is_error: false,
            total_cost_usd: None,
            usage: None,
        }
    }

    /// Create a response with usage statistics.
    pub fn with_usage(text: &str, usage: Usage) -> Self {
        let mut resp = Self::text(text);
        resp.usage = Some(usage);
        resp
    }

    /// Set the session ID.
    pub fn session_id(mut self, id: &str) -> Self {
        self.session_id = id.to_string();
        self
    }

    /// Set total cost.
    pub fn cost(mut self, cost: f64) -> Self {
        self.total_cost_usd = Some(cost);
        self
    }

    /// Convert this response into an `AgentOutput`.
    pub fn into_output(self) -> AgentOutput {
        AgentOutput {
            agent: "mock".to_string(),
            session_id: self.session_id,
            events: self.events,
            result: self.result,
            is_error: self.is_error,
            total_cost_usd: self.total_cost_usd,
            usage: self.usage,
        }
    }
}

/// A mock agent for integration testing.
///
/// Implements the full `Agent` trait using an in-memory response queue
/// instead of spawning a subprocess. Tracks calls for test assertions.
pub struct MockAgent {
    system_prompt: String,
    model: String,
    root: Option<String>,
    skip_permissions: bool,
    output_format: Option<String>,
    add_dirs: Vec<String>,
    max_turns: Option<u32>,
    sandbox: Option<SandboxConfig>,

    /// Queue of responses to return from `run()`. Pops from front.
    responses: Mutex<Vec<MockResponse>>,
    /// Default response when queue is empty.
    default_response: Mutex<MockResponse>,
    /// Number of times `run()` has been called.
    pub run_count: AtomicUsize,
    /// Number of times `run_interactive()` has been called.
    pub interactive_count: AtomicUsize,
    /// Number of times `run_resume()` has been called.
    pub resume_count: AtomicUsize,
    /// The last prompt passed to `run()`.
    pub last_prompt: Mutex<Option<String>>,
    /// All prompts passed to `run()`, in order.
    pub all_prompts: Mutex<Vec<String>>,
    /// Whether `run()` should return an error.
    pub fail_on_run: bool,
    /// The error message for `run()` failures.
    pub run_error_message: String,
    /// Whether `run_interactive()` should return an error.
    pub fail_on_interactive: bool,
    /// Optional delay before returning from `run()`.
    pub delay: Option<Duration>,
}

impl MockAgent {
    /// Create a new mock agent with default settings.
    pub fn new() -> Self {
        Self {
            system_prompt: String::new(),
            model: DEFAULT_MODEL.to_string(),
            root: None,
            skip_permissions: false,
            output_format: None,
            add_dirs: Vec::new(),
            max_turns: None,
            sandbox: None,
            responses: Mutex::new(Vec::new()),
            default_response: Mutex::new(MockResponse::text("")),
            run_count: AtomicUsize::new(0),
            interactive_count: AtomicUsize::new(0),
            resume_count: AtomicUsize::new(0),
            last_prompt: Mutex::new(None),
            all_prompts: Mutex::new(Vec::new()),
            fail_on_run: false,
            run_error_message: "Mock agent run failed".to_string(),
            fail_on_interactive: false,
            delay: None,
        }
    }

    /// Create a `MockAgentBuilder` for fluent configuration.
    pub fn builder() -> MockAgentBuilder {
        MockAgentBuilder::new()
    }

    /// Get the number of times `run()` was called.
    pub fn run_count(&self) -> usize {
        self.run_count.load(Ordering::SeqCst)
    }

    /// Get the number of times `run_interactive()` was called.
    pub fn interactive_count(&self) -> usize {
        self.interactive_count.load(Ordering::SeqCst)
    }

    /// Get the number of times `run_resume()` was called.
    pub fn resume_count(&self) -> usize {
        self.resume_count.load(Ordering::SeqCst)
    }

    /// Get the last prompt that was passed to `run()`.
    pub fn last_prompt(&self) -> Option<String> {
        self.last_prompt.lock().unwrap().clone()
    }

    /// Get all prompts passed to `run()`, in order.
    pub fn all_prompts(&self) -> Vec<String> {
        self.all_prompts.lock().unwrap().clone()
    }

    /// Get the configured max_turns value.
    pub fn max_turns(&self) -> Option<u32> {
        self.max_turns
    }

    /// Get the configured root directory.
    pub fn root(&self) -> Option<&str> {
        self.root.as_deref()
    }

    /// Get whether skip_permissions is enabled.
    pub fn skip_permissions(&self) -> bool {
        self.skip_permissions
    }

    /// Get the configured output format.
    pub fn output_format(&self) -> Option<&str> {
        self.output_format.as_deref()
    }

    /// Get the configured additional directories.
    pub fn add_dirs(&self) -> &[String] {
        &self.add_dirs
    }

    /// Get the configured sandbox.
    pub fn sandbox(&self) -> Option<&SandboxConfig> {
        self.sandbox.as_ref()
    }
}

impl Default for MockAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring a `MockAgent` with specific test behavior.
pub struct MockAgentBuilder {
    responses: Vec<MockResponse>,
    default_response: Option<MockResponse>,
    fail_on_run: bool,
    run_error_message: String,
    fail_on_interactive: bool,
    delay: Option<Duration>,
    model: Option<String>,
    system_prompt: Option<String>,
}

impl MockAgentBuilder {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
            default_response: None,
            fail_on_run: false,
            run_error_message: "Mock agent run failed".to_string(),
            fail_on_interactive: false,
            delay: None,
            model: None,
            system_prompt: None,
        }
    }

    /// Queue a text response.
    pub fn respond_with_text(mut self, text: &str) -> Self {
        self.responses.push(MockResponse::text(text));
        self
    }

    /// Queue an error response.
    pub fn respond_with_error(mut self, message: &str) -> Self {
        self.responses.push(MockResponse::error(message));
        self
    }

    /// Queue a custom response.
    pub fn respond_with(mut self, response: MockResponse) -> Self {
        self.responses.push(response);
        self
    }

    /// Set the default response when the queue is empty.
    pub fn default_response(mut self, response: MockResponse) -> Self {
        self.default_response = Some(response);
        self
    }

    /// Make `run()` always return an error (ignoring the response queue).
    pub fn fail_on_run(mut self, message: &str) -> Self {
        self.fail_on_run = true;
        self.run_error_message = message.to_string();
        self
    }

    /// Make `run_interactive()` return an error.
    pub fn fail_on_interactive(mut self) -> Self {
        self.fail_on_interactive = true;
        self
    }

    /// Add a delay before `run()` returns.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Set the initial model.
    pub fn model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    /// Set the initial system prompt.
    pub fn system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Build the `MockAgent`.
    pub fn build(self) -> MockAgent {
        let mut agent = MockAgent::new();
        *agent.responses.lock().unwrap() = self.responses;
        if let Some(default) = self.default_response {
            *agent.default_response.lock().unwrap() = default;
        }
        agent.fail_on_run = self.fail_on_run;
        agent.run_error_message = self.run_error_message;
        agent.fail_on_interactive = self.fail_on_interactive;
        agent.delay = self.delay;
        if let Some(model) = self.model {
            agent.model = model;
        }
        if let Some(prompt) = self.system_prompt {
            agent.system_prompt = prompt;
        }
        agent
    }
}

impl Default for MockAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for MockAgent {
    fn name(&self) -> &str {
        "mock"
    }

    fn default_model() -> &'static str
    where
        Self: Sized,
    {
        DEFAULT_MODEL
    }

    fn model_for_size(size: ModelSize) -> &'static str
    where
        Self: Sized,
    {
        match size {
            ModelSize::Small => "mock-small",
            ModelSize::Medium => "mock-medium",
            ModelSize::Large => "mock-large",
        }
    }

    fn available_models() -> &'static [&'static str]
    where
        Self: Sized,
    {
        AVAILABLE_MODELS
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }

    fn get_model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }

    fn set_root(&mut self, root: String) {
        self.root = Some(root);
    }

    fn set_skip_permissions(&mut self, skip: bool) {
        self.skip_permissions = skip;
    }

    fn set_output_format(&mut self, format: Option<String>) {
        self.output_format = format;
    }

    fn set_max_turns(&mut self, turns: u32) {
        self.max_turns = Some(turns);
    }

    fn set_sandbox(&mut self, config: SandboxConfig) {
        self.sandbox = Some(config);
    }

    fn set_add_dirs(&mut self, dirs: Vec<String>) {
        self.add_dirs = dirs;
    }

    fn as_any_ref(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn run(&self, prompt: Option<&str>) -> Result<Option<AgentOutput>> {
        self.run_count.fetch_add(1, Ordering::SeqCst);

        // Record the prompt
        if let Some(p) = prompt {
            *self.last_prompt.lock().unwrap() = Some(p.to_string());
            self.all_prompts.lock().unwrap().push(p.to_string());
        }

        // Simulate delay
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        // Check if we should fail
        if self.fail_on_run {
            anyhow::bail!("{}", self.run_error_message);
        }

        // Pop next response from queue, or use default
        let response = {
            let mut queue = self.responses.lock().unwrap();
            if queue.is_empty() {
                self.default_response.lock().unwrap().clone()
            } else {
                queue.remove(0)
            }
        };

        Ok(Some(response.into_output()))
    }

    async fn run_interactive(&self, prompt: Option<&str>) -> Result<()> {
        self.interactive_count.fetch_add(1, Ordering::SeqCst);

        if let Some(p) = prompt {
            *self.last_prompt.lock().unwrap() = Some(p.to_string());
            self.all_prompts.lock().unwrap().push(p.to_string());
        }

        if self.fail_on_interactive {
            anyhow::bail!("Mock agent interactive session failed");
        }

        Ok(())
    }

    async fn run_resume(&self, _session_id: Option<&str>, _last: bool) -> Result<()> {
        self.resume_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn run_resume_with_prompt(
        &self,
        _session_id: &str,
        prompt: &str,
    ) -> Result<Option<AgentOutput>> {
        self.run_count.fetch_add(1, Ordering::SeqCst);
        *self.last_prompt.lock().unwrap() = Some(prompt.to_string());
        self.all_prompts.lock().unwrap().push(prompt.to_string());

        let response = {
            let mut queue = self.responses.lock().unwrap();
            if queue.is_empty() {
                self.default_response.lock().unwrap().clone()
            } else {
                queue.remove(0)
            }
        };

        Ok(Some(response.into_output()))
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

/// Convenience functions for creating common event sequences in tests.
pub mod events {
    use super::*;

    /// Create an init event.
    pub fn init(model: &str) -> Event {
        Event::Init {
            model: model.to_string(),
            tools: vec!["Bash".to_string(), "Read".to_string(), "Write".to_string()],
            working_directory: Some("/tmp/test".to_string()),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create an assistant message event with text content.
    pub fn assistant_message(text: &str) -> Event {
        Event::AssistantMessage {
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            usage: None,
        }
    }

    /// Create an assistant message with usage stats.
    pub fn assistant_message_with_usage(
        text: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Event {
        Event::AssistantMessage {
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            usage: Some(Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens: None,
                cache_creation_tokens: None,
                web_search_requests: None,
                web_fetch_requests: None,
            }),
        }
    }

    /// Create a tool execution event.
    pub fn tool_execution(tool_name: &str, input: &str, output: &str) -> Event {
        Event::ToolExecution {
            tool_name: tool_name.to_string(),
            tool_id: uuid::Uuid::new_v4().to_string(),
            input: serde_json::json!({ "command": input }),
            result: ToolResult {
                success: true,
                output: Some(output.to_string()),
                error: None,
                data: None,
            },
        }
    }

    /// Create a failed tool execution event.
    pub fn tool_execution_failed(tool_name: &str, error: &str) -> Event {
        Event::ToolExecution {
            tool_name: tool_name.to_string(),
            tool_id: uuid::Uuid::new_v4().to_string(),
            input: serde_json::Value::Null,
            result: ToolResult {
                success: false,
                output: None,
                error: Some(error.to_string()),
                data: None,
            },
        }
    }

    /// Create a successful result event.
    pub fn result_success(message: &str) -> Event {
        Event::Result {
            success: true,
            message: Some(message.to_string()),
            duration_ms: Some(100),
            num_turns: Some(1),
        }
    }

    /// Create a user message event.
    pub fn user_message(text: &str) -> Event {
        Event::UserMessage {
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
        }
    }

    /// Create a permission request event.
    pub fn permission_granted(tool_name: &str) -> Event {
        Event::PermissionRequest {
            tool_name: tool_name.to_string(),
            description: format!("Allow {} to execute", tool_name),
            granted: true,
        }
    }
}

#[cfg(test)]
#[path = "mock_tests.rs"]
mod tests;
