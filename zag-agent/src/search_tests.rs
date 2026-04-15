use super::*;
use crate::session_log::{
    AgentLogEvent, LogCompleteness, LogEventKind, LogSourceKind, SessionLogIndex,
    SessionLogIndexEntry, ToolKind,
};
use chrono::Datelike;
use chrono::Timelike;
use std::fs;
use std::io::Write as IoWrite;

// ---------------------------------------------------------------------------
// Helper: build an AgentLogEvent with a given kind
// ---------------------------------------------------------------------------

fn make_event(kind: LogEventKind) -> AgentLogEvent {
    AgentLogEvent {
        seq: 1,
        ts: "2026-03-24T10:00:00Z".to_string(),
        provider: "claude".to_string(),
        wrapper_session_id: "test-session".to_string(),
        provider_session_id: None,
        source_kind: LogSourceKind::Wrapper,
        completeness: LogCompleteness::Full,
        kind,
    }
}

fn make_index_entry(
    session_id: &str,
    provider: &str,
    log_path: &str,
    workspace_path: Option<&str>,
) -> SessionLogIndexEntry {
    SessionLogIndexEntry {
        wrapper_session_id: session_id.to_string(),
        provider: provider.to_string(),
        provider_session_id: None,
        log_path: log_path.to_string(),
        completeness: LogCompleteness::Full,
        started_at: "2026-03-24T10:00:00Z".to_string(),
        ended_at: None,
        workspace_path: workspace_path.map(str::to_string),
        command: Some("exec".to_string()),
        source_paths: vec![],
        backfilled: false,
    }
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("zag-search-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ===========================================================================
// parse_date_arg
// ===========================================================================

#[test]
fn test_parse_date_arg_rfc3339() {
    let dt = parse_date_arg("2024-01-15T10:30:00Z").unwrap();
    assert!(dt.to_rfc3339().starts_with("2024-01-15T10:30:00"));
}

#[test]
fn test_parse_date_arg_date_only() {
    let dt = parse_date_arg("2024-01-15").unwrap();
    assert_eq!(dt.hour(), 0);
    assert_eq!(dt.minute(), 0);
    assert_eq!(dt.day(), 15);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.year(), 2024);
}

#[test]
fn test_parse_date_arg_relative_hours() {
    let before = Utc::now() - Duration::hours(2);
    let dt = parse_date_arg("1h").unwrap();
    // Should be roughly 1 hour ago (between 0h55m and 1h5m ago)
    assert!(dt > before, "parsed date should be after (now - 2h)");
    assert!(dt < Utc::now(), "parsed date should be before now");
}

#[test]
fn test_parse_date_arg_relative_days() {
    let before = Utc::now() - Duration::days(3);
    let dt = parse_date_arg("2d").unwrap();
    assert!(dt > before);
    assert!(dt < Utc::now());
}

#[test]
fn test_parse_date_arg_relative_weeks() {
    let before = Utc::now() - Duration::weeks(4);
    let dt = parse_date_arg("3w").unwrap();
    assert!(dt > before);
    assert!(dt < Utc::now());
}

#[test]
fn test_parse_date_arg_relative_months() {
    let before = Utc::now() - Duration::days(62);
    let dt = parse_date_arg("1m").unwrap();
    assert!(dt > before);
    assert!(dt < Utc::now());
}

#[test]
fn test_parse_date_arg_invalid_unit() {
    let result = parse_date_arg("5x");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unknown time unit"));
}

#[test]
fn test_parse_date_arg_completely_invalid() {
    let result = parse_date_arg("not-a-date");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Cannot parse date")
    );
}

// ===========================================================================
// TextMatcher
// ===========================================================================

#[test]
fn test_text_matcher_none_matches_everything() {
    let query = SearchQuery::new();
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!matcher.has_filter());
    assert!(matcher.is_match("anything at all"));
    assert!(matcher.is_match(""));
}

#[test]
fn test_text_matcher_literal_case_insensitive() {
    let query = SearchQuery {
        text: Some("Hello".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.has_filter());
    assert!(matcher.is_match("say HELLO world"));
    assert!(matcher.is_match("hello"));
    assert!(!matcher.is_match("goodbye"));
}

#[test]
fn test_text_matcher_literal_case_sensitive() {
    // When case_insensitive=false, the needle is stored as-is (not lowercased),
    // but is_match() always lowercases the haystack before checking .contains().
    // This means a mixed-case needle like "Hello" will never match because the
    // lowercased haystack can never contain an uppercase letter.
    // Only a fully lowercase needle matches (effectively: case-insensitive on
    // haystack side, case-preserved on needle side).
    let query = SearchQuery {
        text: Some("hello".to_string()),
        case_insensitive: false,
        use_regex: false,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.has_filter());
    // Lowercase needle matches any-case haystack (haystack is lowercased)
    assert!(matcher.is_match("HELLO WORLD"));
    assert!(matcher.is_match("Hello there"));

    // Mixed-case needle: never matches because haystack is lowercased
    let query2 = SearchQuery {
        text: Some("Hello".to_string()),
        case_insensitive: false,
        use_regex: false,
        ..Default::default()
    };
    let matcher2 = TextMatcher::build(&query2).unwrap();
    assert!(!matcher2.is_match("Hello there"));
    assert!(!matcher2.is_match("HELLO"));
}

#[test]
fn test_text_matcher_regex_case_insensitive() {
    let query = SearchQuery {
        text: Some(r"fn\s+\w+_handler".to_string()),
        case_insensitive: true,
        use_regex: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.has_filter());
    assert!(matcher.is_match("fn my_handler()"));
    assert!(matcher.is_match("FN My_Handler()"));
    assert!(!matcher.is_match("function handler"));
}

#[test]
fn test_text_matcher_regex_case_sensitive() {
    let query = SearchQuery {
        text: Some(r"Error".to_string()),
        case_insensitive: false,
        use_regex: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.is_match("SomeError occurred"));
    assert!(!matcher.is_match("some error occurred"));
}

#[test]
fn test_text_matcher_invalid_regex() {
    let query = SearchQuery {
        text: Some(r"[invalid".to_string()),
        use_regex: true,
        ..Default::default()
    };
    let result = TextMatcher::build(&query);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("Invalid regex"));
}

#[test]
fn test_text_matcher_find_offset() {
    let query = SearchQuery {
        text: Some("world".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let offset = matcher.find_offset("hello world");
    assert_eq!(offset, Some(6));
}

#[test]
fn test_text_matcher_find_offset_no_match() {
    let query = SearchQuery {
        text: Some("xyz".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert_eq!(matcher.find_offset("hello world"), None);
}

#[test]
fn test_text_matcher_find_offset_none() {
    let query = SearchQuery::new();
    let matcher = TextMatcher::build(&query).unwrap();
    assert_eq!(matcher.find_offset("anything"), Some(0));
}

// ===========================================================================
// extract_searchable_text
// ===========================================================================

#[test]
fn test_extract_user_message() {
    let event = make_event(LogEventKind::UserMessage {
        role: "user".to_string(),
        content: "hello world".to_string(),
        message_id: None,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("user"));
    assert!(text.contains("hello world"));
}

#[test]
fn test_extract_assistant_message() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "Here is the answer".to_string(),
        message_id: None,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("Here is the answer"));
}

#[test]
fn test_extract_reasoning() {
    let event = make_event(LogEventKind::Reasoning {
        content: "Let me think about this".to_string(),
        message_id: None,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("Let me think about this"));
}

#[test]
fn test_extract_tool_call() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Bash".to_string(),
        tool_kind: Some(ToolKind::Shell),
        tool_id: Some("t1".to_string()),
        input: Some(serde_json::json!({"command": "ls -la"})),
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("Bash"));
    assert!(text.contains("ls -la"));
}

#[test]
fn test_extract_tool_result() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Read".to_string()),
        tool_kind: Some(ToolKind::FileRead),
        tool_id: None,
        success: Some(true),
        output: Some("file contents".to_string()),
        error: None,
        data: None,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("Read"));
    assert!(text.contains("file contents"));
}

#[test]
fn test_extract_tool_result_with_error() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Write".to_string()),
        tool_kind: None,
        tool_id: None,
        success: Some(false),
        output: None,
        error: Some("permission denied".to_string()),
        data: None,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("permission denied"));
}

#[test]
fn test_extract_session_started() {
    let event = make_event(LogEventKind::SessionStarted {
        command: "exec".to_string(),
        model: Some("opus".to_string()),
        cwd: Some("/home/user".to_string()),
        resumed: false,
        backfilled: false,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("exec"));
    assert!(text.contains("opus"));
    assert!(text.contains("/home/user"));
}

#[test]
fn test_extract_session_ended_with_error() {
    let event = make_event(LogEventKind::SessionEnded {
        success: false,
        error: Some("timeout".to_string()),
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("timeout"));
}

#[test]
fn test_extract_permission() {
    let event = make_event(LogEventKind::Permission {
        tool_name: "Bash".to_string(),
        description: "Run dangerous command".to_string(),
        granted: false,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("Bash"));
    assert!(text.contains("Run dangerous command"));
}

#[test]
fn test_extract_provider_status() {
    let event = make_event(LogEventKind::ProviderStatus {
        message: "Rate limited".to_string(),
        data: None,
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("Rate limited"));
}

#[test]
fn test_extract_stderr() {
    let event = make_event(LogEventKind::Stderr {
        message: "warning: unused variable".to_string(),
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("warning: unused variable"));
}

#[test]
fn test_extract_parse_warning() {
    let event = make_event(LogEventKind::ParseWarning {
        message: "unexpected field".to_string(),
        raw: Some("raw data here".to_string()),
    });
    let text = extract_searchable_text(&event);
    assert!(text.contains("unexpected field"));
    assert!(text.contains("raw data here"));
}

// ===========================================================================
// make_snippet
// ===========================================================================

#[test]
fn test_make_snippet_short_text() {
    let query = SearchQuery::new();
    let matcher = TextMatcher::build(&query).unwrap();
    let snippet = make_snippet("short text", &matcher, 200);
    assert_eq!(snippet, "short text");
    assert!(!snippet.contains("[...]"));
}

#[test]
fn test_make_snippet_long_text_no_filter() {
    let query = SearchQuery::new();
    let matcher = TextMatcher::build(&query).unwrap();
    let long = "a".repeat(500);
    let snippet = make_snippet(&long, &matcher, 200);
    // Should start from beginning (offset=0) and show max_len chars
    assert!(snippet.len() <= 210); // max_len + "[...]" suffix
    assert!(snippet.ends_with("[...]"));
}

#[test]
fn test_make_snippet_long_text_match_in_middle() {
    let query = SearchQuery {
        text: Some("NEEDLE".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let text = format!("{}NEEDLE{}", "x".repeat(300), "y".repeat(300));
    let snippet = make_snippet(&text, &matcher, 200);
    assert!(snippet.contains("NEEDLE") || snippet.contains("needle"));
    assert!(snippet.starts_with("[...]"));
}

// ===========================================================================
// session_matches_query
// ===========================================================================

#[test]
fn test_session_matches_no_filters() {
    let entry = make_index_entry("sess-1", "claude", "/tmp/log.jsonl", None);
    let query = SearchQuery::new();
    assert!(session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_provider_filter() {
    let entry = make_index_entry("sess-1", "claude", "/tmp/log.jsonl", None);
    let query = SearchQuery {
        provider: Some("claude".to_string()),
        ..SearchQuery::new()
    };
    assert!(session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_provider_mismatch() {
    let entry = make_index_entry("sess-1", "claude", "/tmp/log.jsonl", None);
    let query = SearchQuery {
        provider: Some("gemini".to_string()),
        ..SearchQuery::new()
    };
    assert!(!session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_provider_case_insensitive() {
    let entry = make_index_entry("sess-1", "Claude", "/tmp/log.jsonl", None);
    let query = SearchQuery {
        provider: Some("claude".to_string()),
        ..SearchQuery::new()
    };
    assert!(session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_session_id_prefix() {
    let entry = make_index_entry("abc-123-def", "claude", "/tmp/log.jsonl", None);
    let query = SearchQuery {
        session_id: Some("abc-123".to_string()),
        ..SearchQuery::new()
    };
    assert!(session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_session_id_prefix_mismatch() {
    let entry = make_index_entry("abc-123-def", "claude", "/tmp/log.jsonl", None);
    let query = SearchQuery {
        session_id: Some("xyz".to_string()),
        ..SearchQuery::new()
    };
    assert!(!session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_ended_before_from() {
    let mut entry = make_index_entry("sess-1", "claude", "/tmp/log.jsonl", None);
    entry.ended_at = Some("2024-01-01T00:00:00Z".to_string());
    let query = SearchQuery {
        from: Some(
            DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
                .unwrap()
                .into(),
        ),
        ..SearchQuery::new()
    };
    assert!(!session_matches_query(&entry, &query));
}

#[test]
fn test_session_matches_started_after_to() {
    let mut entry = make_index_entry("sess-1", "claude", "/tmp/log.jsonl", None);
    entry.started_at = "2025-01-01T00:00:00Z".to_string();
    let query = SearchQuery {
        to: Some(
            DateTime::parse_from_rfc3339("2024-06-01T00:00:00Z")
                .unwrap()
                .into(),
        ),
        ..SearchQuery::new()
    };
    assert!(!session_matches_query(&entry, &query));
}

// ===========================================================================
// event_matches_query
// ===========================================================================

#[test]
fn test_event_matches_no_filters() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "hello".to_string(),
        message_id: None,
    });
    let query = SearchQuery::new();
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_provider_filter() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "hello".to_string(),
        message_id: None,
    });
    let query = SearchQuery {
        provider: Some("claude".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));

    let query_mismatch = SearchQuery {
        provider: Some("gemini".to_string()),
        ..SearchQuery::new()
    };
    assert!(!event_matches_query(&event, &query_mismatch, &matcher));
}

#[test]
fn test_event_matches_date_range_from() {
    let mut event = make_event(LogEventKind::AssistantMessage {
        content: "hello".to_string(),
        message_id: None,
    });
    event.ts = "2024-06-15T10:00:00Z".to_string();
    let query = SearchQuery {
        from: Some(
            DateTime::parse_from_rfc3339("2024-07-01T00:00:00Z")
                .unwrap()
                .into(),
        ),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_date_range_to() {
    let mut event = make_event(LogEventKind::AssistantMessage {
        content: "hello".to_string(),
        message_id: None,
    });
    event.ts = "2024-08-15T10:00:00Z".to_string();
    let query = SearchQuery {
        to: Some(
            DateTime::parse_from_rfc3339("2024-07-01T00:00:00Z")
                .unwrap()
                .into(),
        ),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_tool_name_filter() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Bash".to_string(),
        tool_kind: Some(ToolKind::Shell),
        tool_id: None,
        input: None,
    });
    let query = SearchQuery {
        tool: Some("bash".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_tool_name_mismatch() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Bash".to_string(),
        tool_kind: Some(ToolKind::Shell),
        tool_id: None,
        input: None,
    });
    let query = SearchQuery {
        tool: Some("Read".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_tool_kind_filter() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Bash".to_string(),
        tool_kind: Some(ToolKind::Shell),
        tool_id: None,
        input: None,
    });
    let query = SearchQuery {
        tool_kind: Some(ToolKind::Shell),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_tool_kind_mismatch() {
    let event = make_event(LogEventKind::ToolCall {
        tool_name: "Read".to_string(),
        tool_kind: Some(ToolKind::FileRead),
        tool_id: None,
        input: None,
    });
    let query = SearchQuery {
        tool_kind: Some(ToolKind::Shell),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_non_tool_excluded_with_tool_filter() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "hello".to_string(),
        message_id: None,
    });
    let query = SearchQuery {
        tool: Some("Bash".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_role_filter_user() {
    let event = make_event(LogEventKind::UserMessage {
        role: "user".to_string(),
        content: "hello".to_string(),
        message_id: None,
    });
    let query = SearchQuery {
        role: Some("user".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_role_filter_excludes_non_message() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "hello".to_string(),
        message_id: None,
    });
    let query = SearchQuery {
        role: Some("user".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_text_filter() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "the quick brown fox".to_string(),
        message_id: None,
    });
    let query = SearchQuery {
        text: Some("brown fox".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_text_filter_mismatch() {
    let event = make_event(LogEventKind::AssistantMessage {
        content: "the quick brown fox".to_string(),
        message_id: None,
    });
    let query = SearchQuery {
        text: Some("lazy dog".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(!event_matches_query(&event, &query, &matcher));
}

#[test]
fn test_event_matches_tool_result_with_tool_filter() {
    let event = make_event(LogEventKind::ToolResult {
        tool_name: Some("Bash".to_string()),
        tool_kind: Some(ToolKind::Shell),
        tool_id: None,
        success: Some(true),
        output: Some("output".to_string()),
        error: None,
        data: None,
    });
    let query = SearchQuery {
        tool: Some("bash".to_string()),
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(event_matches_query(&event, &query, &matcher));
}

// ===========================================================================
// scan_session
// ===========================================================================

fn write_jsonl_events(path: &Path, events: &[AgentLogEvent]) {
    let mut file = fs::File::create(path).unwrap();
    for event in events {
        let json = serde_json::to_string(event).unwrap();
        writeln!(file, "{json}").unwrap();
    }
}

#[test]
fn test_scan_session_with_matches() {
    let dir = TempDir::new("scan-match");
    let log_path = dir.path().join("session.jsonl");

    let events = vec![
        make_event(LogEventKind::UserMessage {
            role: "user".to_string(),
            content: "find the bug".to_string(),
            message_id: None,
        }),
        make_event(LogEventKind::AssistantMessage {
            content: "I found a bug in main.rs".to_string(),
            message_id: None,
        }),
    ];
    write_jsonl_events(&log_path, &events);

    let query = SearchQuery {
        text: Some("bug".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let result = scan_session(&log_path, &query, &matcher).unwrap();
    assert_eq!(result.events_scanned, 2);
    assert_eq!(result.matching_events.len(), 2);
}

#[test]
fn test_scan_session_no_matches() {
    let dir = TempDir::new("scan-no-match");
    let log_path = dir.path().join("session.jsonl");

    let events = vec![make_event(LogEventKind::AssistantMessage {
        content: "hello world".to_string(),
        message_id: None,
    })];
    write_jsonl_events(&log_path, &events);

    let query = SearchQuery {
        text: Some("xyz-not-found".to_string()),
        case_insensitive: true,
        ..Default::default()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let result = scan_session(&log_path, &query, &matcher).unwrap();
    assert_eq!(result.events_scanned, 1);
    assert_eq!(result.matching_events.len(), 0);
}

#[test]
fn test_scan_session_skips_malformed_json() {
    let dir = TempDir::new("scan-malformed");
    let log_path = dir.path().join("session.jsonl");

    let event = make_event(LogEventKind::AssistantMessage {
        content: "valid event".to_string(),
        message_id: None,
    });

    let mut file = fs::File::create(&log_path).unwrap();
    writeln!(file, "not valid json").unwrap();
    writeln!(file, "{}", serde_json::to_string(&event).unwrap()).unwrap();
    writeln!(file).unwrap(); // empty line
    writeln!(file, "{{broken json").unwrap();

    let query = SearchQuery::new();
    let matcher = TextMatcher::build(&query).unwrap();
    let result = scan_session(&log_path, &query, &matcher).unwrap();
    assert_eq!(result.events_scanned, 1); // only the valid event
    assert_eq!(result.matching_events.len(), 1);
}

// ===========================================================================
// search (integration)
// ===========================================================================

/// Build a minimal zag_home structure with a project index and log file.
fn setup_search_fixture(
    name: &str,
    workspace_path: &str,
    events: &[AgentLogEvent],
) -> (TempDir, PathBuf) {
    let dir = TempDir::new(name);
    let project_dir = dir.path().join("projects").join("test-project");
    let logs_dir = project_dir.join("logs");
    let sessions_dir = logs_dir.join("sessions");
    fs::create_dir_all(&sessions_dir).unwrap();

    let log_path = sessions_dir.join("test-session.jsonl");
    write_jsonl_events(&log_path, events);

    let index = SessionLogIndex {
        sessions: vec![SessionLogIndexEntry {
            wrapper_session_id: "test-session".to_string(),
            provider: "claude".to_string(),
            provider_session_id: None,
            log_path: log_path.to_string_lossy().to_string(),
            completeness: LogCompleteness::Full,
            started_at: "2026-03-24T10:00:00Z".to_string(),
            ended_at: None,
            workspace_path: Some(workspace_path.to_string()),
            command: Some("exec".to_string()),
            source_paths: vec![],
            backfilled: false,
        }],
    };
    let index_path = logs_dir.join("index.json");
    fs::write(&index_path, serde_json::to_string(&index).unwrap()).unwrap();

    let cwd = PathBuf::from(workspace_path);
    (dir, cwd)
}

#[test]
fn test_search_finds_matches() {
    let events = vec![
        make_event(LogEventKind::UserMessage {
            role: "user".to_string(),
            content: "fix the login bug".to_string(),
            message_id: None,
        }),
        make_event(LogEventKind::AssistantMessage {
            content: "I fixed the login issue".to_string(),
            message_id: None,
        }),
    ];
    let (dir, cwd) = setup_search_fixture("search-match", "/home/user/project", &events);

    let query = SearchQuery {
        text: Some("login".to_string()),
        case_insensitive: true,
        global: true,
        ..Default::default()
    };
    let results = search(&query, dir.path(), &cwd).unwrap();
    assert_eq!(results.total_sessions_scanned, 1);
    assert_eq!(results.matches.len(), 2);
    assert!(results.matches[0].snippet.contains("login"));
}

#[test]
fn test_search_respects_limit() {
    let events = vec![
        make_event(LogEventKind::UserMessage {
            role: "user".to_string(),
            content: "first match".to_string(),
            message_id: None,
        }),
        make_event(LogEventKind::UserMessage {
            role: "user".to_string(),
            content: "second match".to_string(),
            message_id: None,
        }),
        make_event(LogEventKind::UserMessage {
            role: "user".to_string(),
            content: "third match".to_string(),
            message_id: None,
        }),
    ];
    let (dir, cwd) = setup_search_fixture("search-limit", "/home/user/project", &events);

    let query = SearchQuery {
        text: Some("match".to_string()),
        case_insensitive: true,
        global: true,
        limit: Some(2),
        ..Default::default()
    };
    let results = search(&query, dir.path(), &cwd).unwrap();
    assert_eq!(results.matches.len(), 2);
}

#[test]
fn test_search_scope_excludes_other_workspaces() {
    let events = vec![make_event(LogEventKind::UserMessage {
        role: "user".to_string(),
        content: "hello world".to_string(),
        message_id: None,
    })];
    let (dir, _cwd) = setup_search_fixture("search-scope", "/home/user/other-project", &events);

    // Search from a different cwd (non-global mode should exclude this workspace)
    let query = SearchQuery {
        text: Some("hello".to_string()),
        case_insensitive: true,
        global: false, // scoped to cwd
        ..Default::default()
    };
    let different_cwd = PathBuf::from("/home/user/my-project");
    let results = search(&query, dir.path(), &different_cwd).unwrap();
    assert_eq!(results.matches.len(), 0);
}

#[test]
fn test_search_global_includes_all_workspaces() {
    let events = vec![make_event(LogEventKind::UserMessage {
        role: "user".to_string(),
        content: "hello world".to_string(),
        message_id: None,
    })];
    let (dir, _cwd) = setup_search_fixture("search-global", "/home/user/other-project", &events);

    let query = SearchQuery {
        text: Some("hello".to_string()),
        case_insensitive: true,
        global: true,
        ..Default::default()
    };
    let different_cwd = PathBuf::from("/home/user/my-project");
    let results = search(&query, dir.path(), &different_cwd).unwrap();
    assert_eq!(results.matches.len(), 1);
}

#[test]
fn test_search_empty_projects_dir() {
    let dir = TempDir::new("search-empty");
    let query = SearchQuery::new();
    let cwd = PathBuf::from("/tmp");
    let results = search(&query, dir.path(), &cwd).unwrap();
    assert_eq!(results.total_sessions_scanned, 0);
    assert_eq!(results.matches.len(), 0);
}

#[test]
fn test_search_missing_log_file() {
    let dir = TempDir::new("search-missing-log");
    let project_dir = dir.path().join("projects").join("test-project");
    let logs_dir = project_dir.join("logs");
    fs::create_dir_all(&logs_dir).unwrap();

    let index = SessionLogIndex {
        sessions: vec![SessionLogIndexEntry {
            wrapper_session_id: "sess-1".to_string(),
            provider: "claude".to_string(),
            provider_session_id: None,
            log_path: "/nonexistent/path.jsonl".to_string(),
            completeness: LogCompleteness::Full,
            started_at: "2026-03-24T10:00:00Z".to_string(),
            ended_at: None,
            workspace_path: Some("/tmp".to_string()),
            command: None,
            source_paths: vec![],
            backfilled: false,
        }],
    };
    fs::write(
        logs_dir.join("index.json"),
        serde_json::to_string(&index).unwrap(),
    )
    .unwrap();

    let query = SearchQuery {
        global: true,
        ..SearchQuery::new()
    };
    let results = search(&query, dir.path(), &PathBuf::from("/tmp")).unwrap();
    assert_eq!(results.total_sessions_scanned, 1);
    assert_eq!(results.total_files_missing, 1);
    assert_eq!(results.matches.len(), 0);
}

#[test]
fn test_search_provider_filter() {
    let events = vec![make_event(LogEventKind::AssistantMessage {
        content: "response".to_string(),
        message_id: None,
    })];
    let (dir, cwd) = setup_search_fixture("search-provider", "/home/user/project", &events);

    // Should match: session provider is "claude"
    let query = SearchQuery {
        provider: Some("claude".to_string()),
        global: true,
        ..SearchQuery::new()
    };
    let results = search(&query, dir.path(), &cwd).unwrap();
    assert_eq!(results.matches.len(), 1);

    // Should not match: session provider is "claude", not "gemini"
    let query = SearchQuery {
        provider: Some("gemini".to_string()),
        global: true,
        ..SearchQuery::new()
    };
    let results = search(&query, dir.path(), &cwd).unwrap();
    assert_eq!(results.total_sessions_scanned, 0); // pre-filtered at session level
}

#[test]
fn test_search_query_new_defaults() {
    let q = SearchQuery::new();
    assert!(q.case_insensitive);
    assert!(!q.use_regex);
    assert!(q.text.is_none());
    assert!(q.provider.is_none());
    assert!(q.limit.is_none());
    assert!(!q.global);
}

// ---------------------------------------------------------------------------
// make_snippet unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_make_snippet_short_text_no_ellipsis() {
    let query = SearchQuery {
        text: None,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let snippet = make_snippet("short text", &matcher, 200);
    assert_eq!(snippet, "short text");
}

#[test]
fn test_make_snippet_long_text_with_match() {
    let query = SearchQuery {
        text: Some("needle".to_string()),
        case_insensitive: true,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    // Create a long string with "needle" buried in the middle
    let prefix = "x".repeat(200);
    let suffix = "y".repeat(200);
    let text = format!("{prefix}needle{suffix}");
    let snippet = make_snippet(&text, &matcher, 50);
    assert!(snippet.contains("needle"));
    assert!(snippet.contains("[...]"));
}

#[test]
fn test_make_snippet_match_at_start() {
    let query = SearchQuery {
        text: Some("hello".to_string()),
        case_insensitive: true,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let text = format!("hello{}", "z".repeat(300));
    let snippet = make_snippet(&text, &matcher, 50);
    assert!(snippet.contains("hello"));
    // Should have trailing ellipsis but no leading one
    assert!(snippet.contains("[...]"));
    assert!(!snippet.starts_with("[...]"));
}

#[test]
fn test_make_snippet_match_at_end() {
    let query = SearchQuery {
        text: Some("world".to_string()),
        case_insensitive: true,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let text = format!("{}world", "a".repeat(300));
    let snippet = make_snippet(&text, &matcher, 50);
    assert!(snippet.contains("world"));
    // Should have leading ellipsis
    assert!(snippet.starts_with("[...]"));
}

#[test]
fn test_make_snippet_no_filter_long_text() {
    let query = SearchQuery {
        text: None,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    let text = "b".repeat(500);
    let snippet = make_snippet(&text, &matcher, 50);
    // Snippet should be truncated and have trailing ellipsis
    assert!(snippet.len() < text.len());
    assert!(snippet.contains("[...]"));
}

// ---------------------------------------------------------------------------
// TextMatcher unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_text_matcher_none_has_no_filter() {
    let query = SearchQuery {
        text: None,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.is_match("anything"));
    assert!(matcher.is_match(""));
    assert!(!matcher.has_filter());
}

#[test]
fn test_text_matcher_literal_case_insensitive_match() {
    let query = SearchQuery {
        text: Some("Hello".to_string()),
        case_insensitive: true,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.is_match("hello world"));
    assert!(matcher.is_match("HELLO WORLD"));
    assert!(!matcher.is_match("goodbye"));
    assert!(matcher.has_filter());
}

#[test]
fn test_text_matcher_regex_pattern() {
    let query = SearchQuery {
        text: Some(r"fn\s+\w+".to_string()),
        use_regex: true,
        case_insensitive: false,
        ..SearchQuery::new()
    };
    let matcher = TextMatcher::build(&query).unwrap();
    assert!(matcher.is_match("fn hello_world"));
    assert!(!matcher.is_match("function hello"));
}

#[test]
fn test_text_matcher_invalid_regex_errors() {
    let query = SearchQuery {
        text: Some("[invalid".to_string()),
        use_regex: true,
        ..SearchQuery::new()
    };
    let result = TextMatcher::build(&query);
    assert!(result.is_err());
}
