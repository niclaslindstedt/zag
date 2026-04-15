//! Integration tests exercising the full AgentBuilder → AgentFactory → MockAgent pipeline.
//!
//! These tests only use public API methods. Tests that need to verify internal
//! state downcast to MockAgent via `as_any_ref()`.

use crate::builder::AgentBuilder;
use crate::factory::AgentFactory;
use crate::output::Usage;
use crate::providers::mock::{MockAgent, MockResponse, events};

// ---------------------------------------------------------------------------
// Factory integration (public API only)
// ---------------------------------------------------------------------------

#[test]
fn test_factory_create_mock() {
    let agent = AgentFactory::create("mock", None, None, None, false, vec![]).unwrap();
    assert_eq!(agent.name(), "mock");
    // Model depends on config (may be "mock-medium" if config has model = "medium")
    let model = agent.get_model();
    assert!(
        model.starts_with("mock-"),
        "Expected mock model, got: {model}"
    );
}

#[test]
fn test_factory_create_mock_with_model() {
    let agent = AgentFactory::create(
        "mock",
        None,
        Some("mock-large".to_string()),
        None,
        false,
        vec![],
    )
    .unwrap();
    assert_eq!(agent.get_model(), "mock-large");
}

#[test]
fn test_factory_create_mock_with_size_alias() {
    let agent =
        AgentFactory::create("mock", None, Some("small".to_string()), None, false, vec![]).unwrap();
    assert_eq!(agent.get_model(), "mock-small");
}

#[test]
fn test_factory_create_mock_medium_size() {
    let agent = AgentFactory::create(
        "mock",
        None,
        Some("medium".to_string()),
        None,
        false,
        vec![],
    )
    .unwrap();
    assert_eq!(agent.get_model(), "mock-medium");
}

#[test]
fn test_factory_create_mock_large_size() {
    let agent =
        AgentFactory::create("mock", None, Some("large".to_string()), None, false, vec![]).unwrap();
    assert_eq!(agent.get_model(), "mock-large");
}

#[test]
fn test_factory_create_mock_with_system_prompt() {
    let agent = AgentFactory::create(
        "mock",
        Some("Be helpful".to_string()),
        None,
        None,
        false,
        vec![],
    )
    .unwrap();
    assert_eq!(agent.system_prompt(), "Be helpful");
}

#[test]
fn test_factory_create_mock_with_invalid_model() {
    let result = AgentFactory::create(
        "mock",
        None,
        Some("invalid-model".to_string()),
        None,
        false,
        vec![],
    );
    let err = result.err().expect("Expected an error");
    assert!(err.to_string().contains("Invalid model"));
}

#[test]
fn test_factory_create_mock_with_auto_approve() {
    let agent = AgentFactory::create("mock", None, None, None, true, vec![]).unwrap();
    let mock = agent.as_any_ref().downcast_ref::<MockAgent>().unwrap();
    assert!(mock.skip_permissions());
}

#[test]
fn test_factory_create_mock_with_add_dirs() {
    let agent = AgentFactory::create(
        "mock",
        None,
        None,
        None,
        false,
        vec!["/a".to_string(), "/b".to_string()],
    )
    .unwrap();
    let mock = agent.as_any_ref().downcast_ref::<MockAgent>().unwrap();
    assert_eq!(mock.add_dirs(), &["/a", "/b"]);
}

// ---------------------------------------------------------------------------
// Builder → exec integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_builder_exec_with_mock() {
    let output = AgentBuilder::new()
        .provider("mock")
        .exec("say hello")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
    assert!(!output.is_error);
}

#[tokio::test]
async fn test_builder_exec_mock_with_model() {
    let output = AgentBuilder::new()
        .provider("mock")
        .model("mock-large")
        .exec("test prompt")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_with_size_alias() {
    let output = AgentBuilder::new()
        .provider("mock")
        .model("small")
        .exec("test prompt")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_with_system_prompt() {
    let output = AgentBuilder::new()
        .provider("mock")
        .system_prompt("You are a test assistant")
        .exec("test")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_with_max_turns() {
    let output = AgentBuilder::new()
        .provider("mock")
        .max_turns(5)
        .exec("test")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_json_mode() {
    // JSON mode for non-claude agents should augment the system prompt
    let output = AgentBuilder::new()
        .provider("mock")
        .json()
        .exec("list 3 colors")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_json_schema_valid() {
    // The default mock response is "" which won't parse as valid JSON,
    // so schema validation should fail
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "colors": { "type": "array" }
        },
        "required": ["colors"]
    });

    let result = AgentBuilder::new()
        .provider("mock")
        .json_schema(schema)
        .exec("list colors")
        .await;

    // Default mock response is "" which fails JSON schema validation
    assert!(result.is_err());
}

#[tokio::test]
async fn test_builder_exec_mock_auto_approve() {
    let output = AgentBuilder::new()
        .provider("mock")
        .auto_approve(true)
        .exec("test")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_output_format() {
    let output = AgentBuilder::new()
        .provider("mock")
        .output_format("json")
        .exec("test")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_verbose() {
    let output = AgentBuilder::new()
        .provider("mock")
        .verbose(true)
        .exec("test")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

#[tokio::test]
async fn test_builder_exec_mock_quiet() {
    let output = AgentBuilder::new()
        .provider("mock")
        .quiet(true)
        .exec("test")
        .await
        .unwrap();

    assert_eq!(output.agent, "mock");
}

// ---------------------------------------------------------------------------
// AgentOutput structure tests
// ---------------------------------------------------------------------------

#[test]
fn test_mock_output_from_text() {
    let output = MockResponse::text("hello world").into_output();
    assert_eq!(output.agent, "mock");
    assert_eq!(output.final_result(), Some("hello world"));
    assert!(output.is_success());
    assert!(output.errors().is_empty());
}

#[test]
fn test_mock_output_error() {
    let output = MockResponse::error("something broke").into_output();
    assert!(output.is_error);
    assert_eq!(output.errors().len(), 1);
}

#[test]
fn test_mock_output_with_events() {
    let output = MockResponse::with_events(vec![
        events::init("mock-default"),
        events::assistant_message("I'll help"),
        events::tool_execution("Bash", "echo hi", "hi"),
        events::assistant_message("Done"),
        events::result_success("completed"),
    ])
    .into_output();

    assert_eq!(output.events.len(), 5);
    assert_eq!(output.final_result(), Some("completed"));
    assert_eq!(output.tool_executions().len(), 1);
}

#[test]
fn test_mock_output_with_usage() {
    let usage = Usage {
        input_tokens: 500,
        output_tokens: 200,
        cache_read_tokens: Some(100),
        cache_creation_tokens: Some(50),
        web_search_requests: None,
        web_fetch_requests: None,
    };
    let output = MockResponse::with_usage("result", usage).into_output();
    let u = output.usage.unwrap();
    assert_eq!(u.input_tokens, 500);
    assert_eq!(u.output_tokens, 200);
    assert_eq!(u.cache_read_tokens, Some(100));
}

#[test]
fn test_mock_output_with_cost() {
    let output = MockResponse::text("result").cost(0.05).into_output();
    assert_eq!(output.total_cost_usd, Some(0.05));
}

// ---------------------------------------------------------------------------
// Log entry extraction
// ---------------------------------------------------------------------------

#[test]
fn test_mock_output_log_entries() {
    let output = MockResponse::with_events(vec![
        events::init("mock-default"),
        events::assistant_message("hello"),
        events::tool_execution("Bash", "ls", "file.txt"),
        events::result_success("done"),
    ])
    .into_output();

    let entries = output.to_log_entries(crate::output::LogLevel::Debug);
    assert!(!entries.is_empty());
}
