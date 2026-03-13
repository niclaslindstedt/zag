use super::Codex;

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
