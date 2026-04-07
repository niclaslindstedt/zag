use super::*;

// ---------------------------------------------------------------------------
// MockResponse construction
// ---------------------------------------------------------------------------

#[test]
fn test_mock_response_text() {
    let resp = MockResponse::text("hello");
    assert_eq!(resp.result.as_deref(), Some("hello"));
    assert!(!resp.is_error);
    assert!(!resp.session_id.is_empty());
    assert_eq!(resp.events.len(), 1);
    assert!(matches!(
        &resp.events[0],
        Event::Result { success: true, .. }
    ));
}

#[test]
fn test_mock_response_error() {
    let resp = MockResponse::error("oops");
    assert_eq!(resp.result.as_deref(), Some("oops"));
    assert!(resp.is_error);
    assert_eq!(resp.events.len(), 1);
    assert!(matches!(&resp.events[0], Event::Error { .. }));
}

#[test]
fn test_mock_response_with_events() {
    let evts = vec![
        events::init("mock-default"),
        events::assistant_message("thinking..."),
        events::result_success("done"),
    ];
    let resp = MockResponse::with_events(evts);
    assert_eq!(resp.result.as_deref(), Some("done"));
    assert!(!resp.is_error);
    assert_eq!(resp.events.len(), 3);
}

#[test]
fn test_mock_response_with_usage() {
    let usage = Usage {
        input_tokens: 100,
        output_tokens: 50,
        cache_read_tokens: None,
        cache_creation_tokens: None,
        web_search_requests: None,
        web_fetch_requests: None,
    };
    let resp = MockResponse::with_usage("result", usage);
    assert_eq!(resp.result.as_deref(), Some("result"));
    assert!(resp.usage.is_some());
    assert_eq!(resp.usage.unwrap().input_tokens, 100);
}

#[test]
fn test_mock_response_chained_setters() {
    let resp = MockResponse::text("hello").session_id("ses-123").cost(0.05);
    assert_eq!(resp.session_id, "ses-123");
    assert_eq!(resp.total_cost_usd, Some(0.05));
}

#[test]
fn test_mock_response_into_output() {
    let output = MockResponse::text("result")
        .session_id("ses-456")
        .cost(0.01)
        .into_output();

    assert_eq!(output.agent, "mock");
    assert_eq!(output.session_id, "ses-456");
    assert_eq!(output.result.as_deref(), Some("result"));
    assert!(!output.is_error);
    assert_eq!(output.total_cost_usd, Some(0.01));
}

// ---------------------------------------------------------------------------
// MockAgent defaults
// ---------------------------------------------------------------------------

#[test]
fn test_mock_agent_defaults() {
    let agent = MockAgent::new();
    assert_eq!(agent.name(), "mock");
    assert_eq!(agent.get_model(), "mock-default");
    assert_eq!(agent.system_prompt(), "");
    assert_eq!(agent.run_count(), 0);
    assert_eq!(agent.interactive_count(), 0);
    assert_eq!(agent.resume_count(), 0);
    assert!(agent.last_prompt().is_none());
    assert!(agent.all_prompts().is_empty());
}

#[test]
fn test_mock_agent_model_resolution() {
    assert_eq!(MockAgent::default_model(), "mock-default");
    assert_eq!(MockAgent::model_for_size(ModelSize::Small), "mock-small");
    assert_eq!(MockAgent::model_for_size(ModelSize::Medium), "mock-medium");
    assert_eq!(MockAgent::model_for_size(ModelSize::Large), "mock-large");
}

#[test]
fn test_mock_agent_resolve_model() {
    assert_eq!(MockAgent::resolve_model("small"), "mock-small");
    assert_eq!(MockAgent::resolve_model("medium"), "mock-medium");
    assert_eq!(MockAgent::resolve_model("large"), "mock-large");
    assert_eq!(MockAgent::resolve_model("custom-model"), "custom-model");
}

#[test]
fn test_mock_agent_validate_model() {
    assert!(MockAgent::validate_model("mock-default", "Mock").is_ok());
    assert!(MockAgent::validate_model("mock-small", "Mock").is_ok());
    assert!(MockAgent::validate_model("invalid", "Mock").is_err());
}

#[test]
fn test_mock_agent_available_models() {
    let models = MockAgent::available_models();
    assert!(models.contains(&"mock-default"));
    assert!(models.contains(&"mock-small"));
    assert!(models.contains(&"mock-medium"));
    assert!(models.contains(&"mock-large"));
}

// ---------------------------------------------------------------------------
// MockAgent setters
// ---------------------------------------------------------------------------

#[test]
fn test_mock_agent_set_system_prompt() {
    let mut agent = MockAgent::new();
    agent.set_system_prompt("Be helpful".to_string());
    assert_eq!(agent.system_prompt(), "Be helpful");
}

#[test]
fn test_mock_agent_set_model() {
    let mut agent = MockAgent::new();
    agent.set_model("mock-large".to_string());
    assert_eq!(agent.get_model(), "mock-large");
}

#[test]
fn test_mock_agent_set_root() {
    let mut agent = MockAgent::new();
    agent.set_root("/tmp/project".to_string());
    assert_eq!(agent.root(), Some("/tmp/project"));
}

#[test]
fn test_mock_agent_set_skip_permissions() {
    let mut agent = MockAgent::new();
    assert!(!agent.skip_permissions());
    agent.set_skip_permissions(true);
    assert!(agent.skip_permissions());
}

#[test]
fn test_mock_agent_set_output_format() {
    let mut agent = MockAgent::new();
    agent.set_output_format(Some("json".to_string()));
    assert_eq!(agent.output_format(), Some("json"));
}

#[test]
fn test_mock_agent_set_max_turns() {
    let mut agent = MockAgent::new();
    agent.set_max_turns(5);
    assert_eq!(agent.max_turns(), Some(5));
}

#[test]
fn test_mock_agent_set_add_dirs() {
    let mut agent = MockAgent::new();
    agent.set_add_dirs(vec!["/a".to_string(), "/b".to_string()]);
    assert_eq!(agent.add_dirs(), &["/a", "/b"]);
}

// ---------------------------------------------------------------------------
// MockAgent run behavior
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mock_agent_run_returns_queued_response() {
    let agent = MockAgent::builder()
        .respond_with_text("first")
        .respond_with_text("second")
        .build();

    let out1 = agent.run(Some("prompt1")).await.unwrap().unwrap();
    assert_eq!(out1.result.as_deref(), Some("first"));

    let out2 = agent.run(Some("prompt2")).await.unwrap().unwrap();
    assert_eq!(out2.result.as_deref(), Some("second"));

    assert_eq!(agent.run_count(), 2);
}

#[tokio::test]
async fn test_mock_agent_run_uses_default_when_queue_empty() {
    let agent = MockAgent::builder()
        .default_response(MockResponse::text("default"))
        .build();

    let out = agent.run(Some("anything")).await.unwrap().unwrap();
    assert_eq!(out.result.as_deref(), Some("default"));
}

#[tokio::test]
async fn test_mock_agent_run_captures_prompt() {
    let agent = MockAgent::new();
    agent.run(Some("hello world")).await.unwrap();
    assert_eq!(agent.last_prompt().as_deref(), Some("hello world"));
    assert_eq!(agent.all_prompts(), vec!["hello world"]);
}

#[tokio::test]
async fn test_mock_agent_run_captures_multiple_prompts() {
    let agent = MockAgent::new();
    agent.run(Some("first")).await.unwrap();
    agent.run(Some("second")).await.unwrap();
    agent.run(Some("third")).await.unwrap();

    assert_eq!(agent.last_prompt().as_deref(), Some("third"));
    assert_eq!(agent.all_prompts(), vec!["first", "second", "third"]);
    assert_eq!(agent.run_count(), 3);
}

#[tokio::test]
async fn test_mock_agent_run_none_prompt() {
    let agent = MockAgent::new();
    let out = agent.run(None).await.unwrap();
    assert!(out.is_some());
    assert!(agent.last_prompt().is_none());
}

#[tokio::test]
async fn test_mock_agent_run_fail() {
    let agent = MockAgent::builder().fail_on_run("custom error").build();

    let result = agent.run(Some("prompt")).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("custom error"));
    // run_count is still incremented
    assert_eq!(agent.run_count(), 1);
}

#[tokio::test]
async fn test_mock_agent_run_with_delay() {
    let agent = MockAgent::builder()
        .respond_with_text("delayed")
        .with_delay(Duration::from_millis(50))
        .build();

    let start = std::time::Instant::now();
    let out = agent.run(Some("prompt")).await.unwrap().unwrap();
    let elapsed = start.elapsed();

    assert_eq!(out.result.as_deref(), Some("delayed"));
    assert!(elapsed >= Duration::from_millis(40)); // Allow some tolerance
}

// ---------------------------------------------------------------------------
// MockAgent interactive and resume
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mock_agent_run_interactive() {
    let agent = MockAgent::new();
    agent.run_interactive(Some("prompt")).await.unwrap();
    assert_eq!(agent.interactive_count(), 1);
    assert_eq!(agent.last_prompt().as_deref(), Some("prompt"));
}

#[tokio::test]
async fn test_mock_agent_run_interactive_fail() {
    let agent = MockAgent::builder().fail_on_interactive().build();
    let result = agent.run_interactive(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mock_agent_run_resume() {
    let agent = MockAgent::new();
    agent.run_resume(Some("session-123"), false).await.unwrap();
    assert_eq!(agent.resume_count(), 1);
}

#[tokio::test]
async fn test_mock_agent_run_resume_with_prompt() {
    let agent = MockAgent::builder().respond_with_text("resumed").build();

    let out = agent
        .run_resume_with_prompt("session-123", "continue")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(out.result.as_deref(), Some("resumed"));
    assert_eq!(agent.last_prompt().as_deref(), Some("continue"));
}

// ---------------------------------------------------------------------------
// MockAgent cleanup
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mock_agent_cleanup() {
    let agent = MockAgent::new();
    assert!(agent.cleanup().await.is_ok());
}

// ---------------------------------------------------------------------------
// MockAgent as_any downcasting
// ---------------------------------------------------------------------------

#[test]
fn test_mock_agent_downcast() {
    let agent = MockAgent::new();
    let any_ref = agent.as_any_ref();
    assert!(any_ref.downcast_ref::<MockAgent>().is_some());
}

#[test]
fn test_mock_agent_downcast_mut() {
    let mut agent = MockAgent::new();
    let any_mut = agent.as_any_mut();
    assert!(any_mut.downcast_mut::<MockAgent>().is_some());
}

// ---------------------------------------------------------------------------
// MockAgentBuilder
// ---------------------------------------------------------------------------

#[test]
fn test_mock_agent_builder_defaults() {
    let agent = MockAgentBuilder::new().build();
    assert_eq!(agent.get_model(), "mock-default");
    assert_eq!(agent.system_prompt(), "");
    assert!(!agent.fail_on_run);
    assert!(!agent.fail_on_interactive);
    assert!(agent.delay.is_none());
}

#[test]
fn test_mock_agent_builder_model() {
    let agent = MockAgent::builder().model("custom").build();
    assert_eq!(agent.get_model(), "custom");
}

#[test]
fn test_mock_agent_builder_system_prompt() {
    let agent = MockAgent::builder().system_prompt("be concise").build();
    assert_eq!(agent.system_prompt(), "be concise");
}

#[test]
fn test_mock_agent_builder_default_impl() {
    let builder = MockAgentBuilder::default();
    let agent = builder.build();
    assert_eq!(agent.name(), "mock");
}

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

#[test]
fn test_events_init() {
    let event = events::init("mock-default");
    assert!(matches!(event, Event::Init { model, .. } if model == "mock-default"));
}

#[test]
fn test_events_assistant_message() {
    let event = events::assistant_message("hello");
    if let Event::AssistantMessage { content, usage, .. } = event {
        assert_eq!(content.len(), 1);
        assert!(matches!(&content[0], ContentBlock::Text { text } if text == "hello"));
        assert!(usage.is_none());
    } else {
        panic!("Expected AssistantMessage");
    }
}

#[test]
fn test_events_assistant_message_with_usage() {
    let event = events::assistant_message_with_usage("hello", 100, 50);
    if let Event::AssistantMessage { usage, .. } = event {
        let u = usage.unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.output_tokens, 50);
    } else {
        panic!("Expected AssistantMessage");
    }
}

#[test]
fn test_events_tool_execution() {
    let event = events::tool_execution("Bash", "ls", "file.txt");
    if let Event::ToolExecution {
        tool_name, result, ..
    } = event
    {
        assert_eq!(tool_name, "Bash");
        assert!(result.success);
        assert_eq!(result.output.as_deref(), Some("file.txt"));
    } else {
        panic!("Expected ToolExecution");
    }
}

#[test]
fn test_events_tool_execution_failed() {
    let event = events::tool_execution_failed("Bash", "command not found");
    if let Event::ToolExecution { result, .. } = event {
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("command not found"));
    } else {
        panic!("Expected ToolExecution");
    }
}

#[test]
fn test_events_user_message() {
    let event = events::user_message("hello");
    assert!(matches!(event, Event::UserMessage { .. }));
}

#[test]
fn test_events_permission_granted() {
    let event = events::permission_granted("Bash");
    if let Event::PermissionRequest {
        tool_name, granted, ..
    } = event
    {
        assert_eq!(tool_name, "Bash");
        assert!(granted);
    } else {
        panic!("Expected PermissionRequest");
    }
}

// ---------------------------------------------------------------------------
// MockResponse with_events extracts result from Result event
// ---------------------------------------------------------------------------

#[test]
fn test_mock_response_with_events_no_result() {
    let evts = vec![events::assistant_message("just text")];
    let resp = MockResponse::with_events(evts);
    assert!(resp.result.is_none()); // No Result event, so no result text
}

#[tokio::test]
async fn test_mock_agent_with_custom_events() {
    let evts = vec![
        events::init("mock-large"),
        events::assistant_message("I'll help you"),
        events::tool_execution("Bash", "echo hello", "hello"),
        events::assistant_message("Done!"),
        events::result_success("Task completed"),
    ];

    let agent = MockAgent::builder()
        .respond_with(MockResponse::with_events(evts))
        .build();

    let output = agent.run(Some("do something")).await.unwrap().unwrap();
    assert_eq!(output.events.len(), 5);
    assert_eq!(output.result.as_deref(), Some("Task completed"));
    assert!(!output.is_error);
}
