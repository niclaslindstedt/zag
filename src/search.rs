use crate::config::Config;
use anyhow::Result;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use zag::search::{SearchMatch, SearchQuery, SearchResults, parse_date_arg, search as lib_search};
use zag::session_log::{AgentLogEvent, LogEventKind};

pub struct SearchCommandArgs {
    pub query: Option<String>,
    pub use_regex: bool,
    pub case_sensitive: bool,
    pub provider: Option<String>,
    pub role: Option<String>,
    pub tool: Option<String>,
    pub tool_kind: Option<zag::session_log::ToolKind>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub session: Option<String>,
    pub tag: Option<String>,
    pub global: bool,
    pub json: bool,
    pub count: bool,
    pub limit: Option<usize>,
    pub root: Option<String>,
}

pub fn run_search_command(args: SearchCommandArgs, quiet: bool) -> Result<()> {
    // Parse date arguments early so errors surface before scanning.
    let from = args.from.as_deref().map(parse_date_arg).transpose()?;
    let to = args.to.as_deref().map(parse_date_arg).transpose()?;

    let query = SearchQuery {
        text: args.query,
        use_regex: args.use_regex,
        case_insensitive: !args.case_sensitive,
        provider: args.provider,
        role: args.role,
        tool: args.tool,
        tool_kind: args.tool_kind,
        from,
        to,
        session_id: args.session,
        tag: args.tag,
        global: args.global,
        limit: args.limit,
    };

    let zag_home = Config::global_base_dir();
    let cwd: PathBuf = args
        .root
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let results = lib_search(&query, &zag_home, &cwd)?;

    if args.count {
        println!("{}", results.matches.len());
        return Ok(());
    }

    if args.json {
        for m in &results.matches {
            println!("{}", serde_json::to_string(m)?);
        }
        return Ok(());
    }

    print_human_readable(&results, quiet);
    Ok(())
}

fn print_human_readable(results: &SearchResults, quiet: bool) {
    let use_color = !quiet && env::var("NO_COLOR").is_err();

    if results.matches.is_empty() {
        println!("No matches found.");
        return;
    }

    // Group matches by session_id preserving first-seen order.
    let mut session_order: Vec<String> = Vec::new();
    let mut by_session: BTreeMap<String, Vec<&SearchMatch>> = BTreeMap::new();

    for m in &results.matches {
        by_session
            .entry(m.session_id.clone())
            .or_insert_with(|| {
                session_order.push(m.session_id.clone());
                Vec::new()
            })
            .push(m);
    }

    for session_id in &session_order {
        let matches = &by_session[session_id];
        let first = matches[0];

        // Session header
        let short_id = &session_id[..session_id.len().min(8)];
        let started = format_timestamp(&first.started_at);
        let workspace = first.workspace_path.as_deref().unwrap_or("(unknown)");

        if use_color {
            println!(
                "\x1b[1mSession:\x1b[0m \x1b[33m{}\x1b[0m  [\x1b[36m{}\x1b[0m]  {}  {}",
                short_id, first.provider, started, workspace
            );
        } else {
            println!(
                "Session: {}  [{}]  {}  {}",
                short_id, first.provider, started, workspace
            );
        }

        if let Some(ref cmd) = first.command
            && !cmd.is_empty()
        {
            let truncated = if cmd.len() > 120 {
                format!("{}...", &cmd[..120])
            } else {
                cmd.clone()
            };
            println!("Command: \"{}\"", truncated);
        }
        println!();

        for m in matches.iter() {
            print_event_match(m, use_color);
        }

        let sep = format!(
            "{} {} match{} {}",
            "\u{2500}".repeat(3),
            matches.len(),
            if matches.len() == 1 { "" } else { "es" },
            "\u{2500}".repeat(60),
        );
        println!("{}", sep);
        println!();
    }

    println!(
        "Found {} match{} in {} session{}  (scanned {} session{}, {} event{})",
        results.matches.len(),
        if results.matches.len() == 1 { "" } else { "es" },
        session_order.len(),
        if session_order.len() == 1 { "" } else { "s" },
        results.total_sessions_scanned,
        if results.total_sessions_scanned == 1 {
            ""
        } else {
            "s"
        },
        results.total_events_scanned,
        if results.total_events_scanned == 1 {
            ""
        } else {
            "s"
        },
    );

    if results.total_files_missing > 0 {
        println!(
            "  ({} log file{} referenced but not found on disk)",
            results.total_files_missing,
            if results.total_files_missing == 1 {
                ""
            } else {
                "s"
            },
        );
    }
}

fn print_event_match(m: &SearchMatch, use_color: bool) {
    let label = event_kind_label(&m.event);
    let ts = format_time_only(&m.event.ts);

    if use_color {
        println!(
            "  \x1b[2m[seq:{}]\x1b[0m  \x1b[1m{}\x1b[0m  \x1b[2m{}\x1b[0m",
            m.event.seq, label, ts
        );
    } else {
        println!("  [seq:{}]  {}  {}", m.event.seq, label, ts);
    }

    if !m.snippet.is_empty() {
        // Indent each line of the snippet
        for line in m.snippet.lines() {
            println!("    {}", line);
        }
    }
    println!();
}

fn event_kind_label(event: &AgentLogEvent) -> String {
    match &event.kind {
        LogEventKind::SessionStarted { .. } => "SessionStarted".to_string(),
        LogEventKind::SessionEnded { .. } => "SessionEnded".to_string(),
        LogEventKind::SessionCleared { .. } => "SessionCleared".to_string(),
        LogEventKind::UserMessage { .. } => "UserMessage".to_string(),
        LogEventKind::AssistantMessage { .. } => "AssistantMessage".to_string(),
        LogEventKind::Reasoning { .. } => "Reasoning".to_string(),
        LogEventKind::ToolCall { tool_name, .. } => format!("ToolCall ({})", tool_name),
        LogEventKind::ToolResult { tool_name, .. } => {
            let name = tool_name.as_deref().unwrap_or("?");
            format!("ToolResult ({})", name)
        }
        LogEventKind::Permission { tool_name, .. } => format!("Permission ({})", tool_name),
        LogEventKind::ProviderStatus { .. } => "ProviderStatus".to_string(),
        LogEventKind::Stderr { .. } => "Stderr".to_string(),
        LogEventKind::ParseWarning { .. } => "ParseWarning".to_string(),
        LogEventKind::Heartbeat { .. } => "Heartbeat".to_string(),
        LogEventKind::Usage { .. } => "Usage".to_string(),
        LogEventKind::UserEvent { level, .. } => format!("UserEvent ({})", level),
    }
}

fn format_timestamp(ts: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        ts.to_string()
    }
}

fn format_time_only(ts: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        dt.format("%H:%M:%S").to_string()
    } else {
        ts.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp_valid() {
        let result = format_timestamp("2024-06-15T10:30:00Z");
        assert!(result.contains("2024"));
        assert!(result.contains("06"));
        assert!(result.contains("15"));
    }

    #[test]
    fn test_format_timestamp_invalid() {
        let result = format_timestamp("not-a-date");
        assert_eq!(result, "not-a-date");
    }

    #[test]
    fn test_format_time_only_valid() {
        let result = format_time_only("2024-06-15T10:30:45Z");
        assert!(result.contains("30"));
        assert!(result.contains("45"));
    }

    #[test]
    fn test_format_time_only_invalid() {
        let result = format_time_only("invalid");
        assert_eq!(result, "invalid");
    }

    #[test]
    fn test_event_kind_label_user_message() {
        let event = AgentLogEvent {
            seq: 1,
            ts: "2026-01-01T00:00:00Z".to_string(),
            provider: "claude".to_string(),
            wrapper_session_id: "s1".to_string(),
            provider_session_id: None,
            source_kind: zag::session_log::LogSourceKind::Wrapper,
            completeness: zag::session_log::LogCompleteness::Full,
            kind: LogEventKind::UserMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
                message_id: None,
            },
        };
        assert_eq!(event_kind_label(&event), "UserMessage");
    }

    #[test]
    fn test_event_kind_label_tool_call() {
        let event = AgentLogEvent {
            seq: 1,
            ts: "2026-01-01T00:00:00Z".to_string(),
            provider: "claude".to_string(),
            wrapper_session_id: "s1".to_string(),
            provider_session_id: None,
            source_kind: zag::session_log::LogSourceKind::Wrapper,
            completeness: zag::session_log::LogCompleteness::Full,
            kind: LogEventKind::ToolCall {
                tool_name: "Bash".to_string(),
                tool_kind: None,
                tool_id: None,
                input: None,
            },
        };
        let label = event_kind_label(&event);
        assert!(label.contains("ToolCall"));
        assert!(label.contains("Bash"));
    }

    #[test]
    fn test_event_kind_label_session_started() {
        let event = AgentLogEvent {
            seq: 1,
            ts: "2026-01-01T00:00:00Z".to_string(),
            provider: "claude".to_string(),
            wrapper_session_id: "s1".to_string(),
            provider_session_id: None,
            source_kind: zag::session_log::LogSourceKind::Wrapper,
            completeness: zag::session_log::LogCompleteness::Full,
            kind: LogEventKind::SessionStarted {
                command: "run".to_string(),
                model: None,
                cwd: None,
                resumed: false,
                backfilled: false,
            },
        };
        assert_eq!(event_kind_label(&event), "SessionStarted");
    }
}
