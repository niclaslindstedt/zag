use super::Codex;
use crate::sandbox::SandboxConfig;

#[test]
fn test_parse_ndjson_extracts_thread_id() {
    let raw = r#"{"type":"thread.started","thread_id":"019ce6a3-7c5d-7672-97f0-36b6e4d3e945"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"hello"}}
{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50}}"#;

    let (thread_id, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(
        thread_id.as_deref(),
        Some("019ce6a3-7c5d-7672-97f0-36b6e4d3e945")
    );
    assert_eq!(text.as_deref(), Some("hello"));
}

#[test]
fn test_parse_ndjson_extracts_agent_message() {
    let raw = r#"{"type":"thread.started","thread_id":"abc123"}
{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"thinking..."}}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"[\"Python\",\"JavaScript\",\"Java\"]"}}
{"type":"turn.completed","usage":{}}"#;

    let (thread_id, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(thread_id.as_deref(), Some("abc123"));
    assert_eq!(
        text.as_deref(),
        Some("[\"Python\",\"JavaScript\",\"Java\"]")
    );
}

#[test]
fn test_parse_ndjson_skips_non_agent_messages() {
    let raw = r#"{"type":"thread.started","thread_id":"t1"}
{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"thinking"}}
{"type":"item.completed","item":{"id":"item_1","type":"web_search","query":"test"}}
{"type":"item.completed","item":{"id":"item_2","type":"agent_message","text":"result"}}
{"type":"turn.completed","usage":{}}"#;

    let (_, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(text.as_deref(), Some("result"));
}

#[test]
fn test_parse_ndjson_concatenates_multiple_agent_messages() {
    let raw = r#"{"type":"thread.started","thread_id":"t1"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"first"}}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"second"}}
{"type":"turn.completed","usage":{}}"#;

    let (_, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(text.as_deref(), Some("first\nsecond"));
}

#[test]
fn test_parse_ndjson_empty_input() {
    let (thread_id, text) = Codex::parse_ndjson_output("");
    assert!(thread_id.is_none());
    assert!(text.is_none());
}

#[test]
fn test_parse_ndjson_no_agent_message() {
    let raw = r#"{"type":"thread.started","thread_id":"t1"}
{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"thinking"}}
{"type":"turn.completed","usage":{}}"#;

    let (thread_id, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(thread_id.as_deref(), Some("t1"));
    assert!(text.is_none());
}

#[test]
fn test_parse_ndjson_invalid_json_lines_skipped() {
    let raw = r#"{"type":"thread.started","thread_id":"t1"}
not valid json
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"ok"}}
also not json"#;

    let (thread_id, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(thread_id.as_deref(), Some("t1"));
    assert_eq!(text.as_deref(), Some("ok"));
}

#[test]
fn test_build_output_plain_text() {
    let codex = Codex::new();
    let output = codex.build_output("hello world");
    assert_eq!(output.result.as_deref(), Some("hello world"));
    assert!(output.session_id.is_empty());
}

#[test]
fn test_build_output_json_mode_parses_ndjson() {
    let mut codex = Codex::new();
    codex.output_format = Some("json".to_string());

    let raw = r#"{"type":"thread.started","thread_id":"tid-123"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"{\"colors\":[\"red\",\"blue\"]}"}}
{"type":"turn.completed","usage":{}}"#;

    let output = codex.build_output(raw);
    assert_eq!(
        output.result.as_deref(),
        Some("{\"colors\":[\"red\",\"blue\"]}")
    );
    assert_eq!(output.session_id, "tid-123");
}

#[test]
fn test_build_run_args_non_interactive() {
    let mut codex = Codex::new();
    codex.model = "gpt-5.4".to_string();
    codex.root = Some("/project".to_string());

    let args = codex.build_run_args(false, Some("hello"));
    assert!(args.contains(&"exec".to_string()));
    assert!(args.contains(&"--skip-git-repo-check".to_string()));
    assert!(args.contains(&"--cd".to_string()));
    assert!(args.contains(&"/project".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gpt-5.4".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive() {
    let codex = Codex::new();
    let args = codex.build_run_args(true, Some("hello"));
    assert!(!args.contains(&"exec".to_string()));
    assert!(!args.contains(&"--skip-git-repo-check".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_sandbox_skips_cd() {
    let mut codex = Codex::new();
    codex.root = Some("/project".to_string());
    codex.sandbox = Some(SandboxConfig {
        name: "test".to_string(),
        template: "docker/sandbox-templates:codex".to_string(),
        workspace: "/workspace".to_string(),
    });

    let args = codex.build_run_args(false, Some("hello"));
    assert!(!args.contains(&"--cd".to_string()));
    assert!(!args.contains(&"/project".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let codex = Codex::new();
    let cmd = codex.make_command(vec!["exec".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "codex");
}

#[test]
fn test_make_command_with_sandbox() {
    let mut codex = Codex::new();
    codex.sandbox = Some(SandboxConfig {
        name: "sandbox-test".to_string(),
        template: "docker/sandbox-templates:codex".to_string(),
        workspace: "/workspace".to_string(),
    });

    let cmd = codex.make_command(vec!["exec".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"sandbox-test"));
    assert!(args.contains(&"exec"));
    assert!(args.contains(&"hello"));
}

#[test]
fn test_build_run_args_max_turns() {
    let mut codex = Codex::new();
    codex.max_turns = Some(5);

    let args = codex.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--max-turns".to_string()));
    assert!(args.contains(&"5".to_string()));
}

#[test]
fn test_build_run_args_no_max_turns_by_default() {
    let codex = Codex::new();
    let args = codex.build_run_args(false, Some("hello"));
    assert!(!args.contains(&"--max-turns".to_string()));
}

#[test]
fn test_build_run_args_full_auto() {
    let mut codex = Codex::new();
    codex.skip_permissions = true;

    let args = codex.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--full-auto".to_string()));
    assert!(!args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
    assert!(!args.contains(&"danger-full-access".to_string()));
}

#[test]
fn test_build_run_args_ephemeral() {
    let mut codex = Codex::new();
    codex.set_ephemeral(true);

    let args = codex.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--ephemeral".to_string()));
}

#[test]
fn test_build_run_args_ephemeral_not_in_interactive() {
    let mut codex = Codex::new();
    codex.set_ephemeral(true);

    let args = codex.build_run_args(true, Some("hello"));
    assert!(!args.contains(&"--ephemeral".to_string()));
}

#[test]
fn test_parse_ndjson_turn_failed() {
    let raw = r#"{"type":"thread.started","thread_id":"t1"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"working on it"}}
{"type":"turn.failed","error":"rate limit exceeded"}"#;

    let (thread_id, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(thread_id.as_deref(), Some("t1"));
    assert_eq!(
        text.as_deref(),
        Some("working on it\n[turn failed: rate limit exceeded]")
    );
}

#[test]
fn test_build_run_args_output_schema() {
    let mut codex = Codex::new();
    codex.set_output_schema(Some("/path/to/schema.json".to_string()));

    let args = codex.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--output-schema".to_string()));
    assert!(args.contains(&"/path/to/schema.json".to_string()));
}

#[test]
fn test_build_run_args_output_schema_not_in_interactive() {
    let mut codex = Codex::new();
    codex.set_output_schema(Some("/path/to/schema.json".to_string()));

    let args = codex.build_run_args(true, Some("hello"));
    assert!(!args.contains(&"--output-schema".to_string()));
}

#[test]
fn test_parse_ndjson_turn_failed_unknown_error() {
    let raw = r#"{"type":"turn.failed"}"#;

    let (_, text) = Codex::parse_ndjson_output(raw);
    assert_eq!(text.as_deref(), Some("[turn failed: unknown error]"));
}
