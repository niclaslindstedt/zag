use crate::session_log::{
    AgentLogEvent, LogEventKind, SessionLogIndex, SessionLogIndexEntry, ToolKind,
};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Query parameters for searching session logs.
#[derive(Debug, Default)]
pub struct SearchQuery {
    /// Full-text pattern (literal substring or regex). None matches all events.
    pub text: Option<String>,
    /// Case-insensitive match (default: true).
    pub case_insensitive: bool,
    /// Treat `text` as a regular expression (default: false → literal substring).
    pub use_regex: bool,
    /// Filter by provider name (case-insensitive).
    pub provider: Option<String>,
    /// Filter by message role — only applies to `UserMessage` events.
    pub role: Option<String>,
    /// Filter by tool name (case-insensitive substring) — only applies to tool events.
    pub tool: Option<String>,
    /// Filter by tool kind — only applies to `ToolCall`/`ToolResult` events.
    pub tool_kind: Option<ToolKind>,
    /// Show only events at or after this timestamp.
    pub from: Option<DateTime<Utc>>,
    /// Show only events at or before this timestamp.
    pub to: Option<DateTime<Utc>>,
    /// Restrict search to a specific session ID (prefix match).
    pub session_id: Option<String>,
    /// Filter by session tag (exact match, case-insensitive).
    pub tag: Option<String>,
    /// Search all sessions across all projects (default: current project and sub-projects).
    pub global: bool,
    /// Stop after collecting this many matches.
    pub limit: Option<usize>,
}

impl SearchQuery {
    pub fn new() -> Self {
        Self {
            case_insensitive: true,
            ..Default::default()
        }
    }
}

/// A single event that matched the search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub session_id: String,
    pub provider: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub workspace_path: Option<String>,
    pub command: Option<String>,
    pub event: AgentLogEvent,
    /// Short excerpt (~200 chars) of the matched text.
    pub snippet: String,
}

/// Aggregate results from a search.
#[derive(Debug, Default)]
pub struct SearchResults {
    pub total_sessions_scanned: usize,
    pub total_events_scanned: usize,
    pub total_files_missing: usize,
    pub matches: Vec<SearchMatch>,
}

// ---------------------------------------------------------------------------
// Date parsing
// ---------------------------------------------------------------------------

/// Parse a date/time string for `--from` / `--to` filters.
///
/// Accepted formats:
/// - RFC 3339 (e.g. `2024-01-15T10:30:00Z`)
/// - Date only (e.g. `2024-01-15`) — interpreted as start of day UTC
/// - Relative offset from now: `1h`, `2d`, `3w`, `1m` (hours/days/weeks/months)
pub fn parse_date_arg(s: &str) -> Result<DateTime<Utc>> {
    // Try RFC 3339 first.
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try date-only (YYYY-MM-DD).
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = date
            .and_hms_opt(0, 0, 0)
            .expect("midnight is always valid")
            .and_utc();
        return Ok(dt);
    }

    // Try relative offset: leading digits followed by a unit character.
    let s_trimmed = s.trim();
    if !s_trimmed.is_empty() {
        let unit = s_trimmed.chars().last().unwrap();
        let digits = &s_trimmed[..s_trimmed.len() - unit.len_utf8()];
        if let Ok(n) = digits.parse::<i64>() {
            let delta = match unit {
                'h' => Duration::hours(n),
                'd' => Duration::days(n),
                'w' => Duration::weeks(n),
                'm' => Duration::days(n * 30),
                _ => bail!(
                    "Unknown time unit '{}'. Use h (hours), d (days), w (weeks), or m (months).",
                    unit
                ),
            };
            return Ok(Utc::now() - delta);
        }
    }

    bail!(
        "Cannot parse date '{}'. Use RFC 3339 (2024-01-15T10:30:00Z), date only (2024-01-15), or relative (1h, 2d, 3w, 1m).",
        s
    )
}

// ---------------------------------------------------------------------------
// Text matcher
// ---------------------------------------------------------------------------

enum TextMatcher {
    /// No text filter — everything matches.
    None,
    /// Case-insensitive literal substring.
    Literal(String),
    /// Compiled regex.
    Pattern(Regex),
}

impl TextMatcher {
    fn build(query: &SearchQuery) -> Result<Self> {
        let Some(ref text) = query.text else {
            return Ok(Self::None);
        };
        if query.use_regex {
            let pattern = if query.case_insensitive {
                format!("(?i){}", text)
            } else {
                text.clone()
            };
            let re = Regex::new(&pattern)
                .with_context(|| format!("Invalid regex pattern: '{}'", text))?;
            Ok(Self::Pattern(re))
        } else if query.case_insensitive {
            Ok(Self::Literal(text.to_lowercase()))
        } else {
            Ok(Self::Literal(text.clone()))
        }
    }

    fn is_match(&self, haystack: &str) -> bool {
        match self {
            Self::None => true,
            Self::Literal(needle) => haystack.to_lowercase().contains(needle.as_str()),
            Self::Pattern(re) => re.is_match(haystack),
        }
    }

    fn find_offset(&self, haystack: &str) -> Option<usize> {
        match self {
            Self::None => Some(0),
            Self::Literal(needle) => haystack.to_lowercase().find(needle.as_str()),
            Self::Pattern(re) => re.find(haystack).map(|m| m.start()),
        }
    }

    fn has_filter(&self) -> bool {
        !matches!(self, Self::None)
    }
}

// ---------------------------------------------------------------------------
// Content extraction
// ---------------------------------------------------------------------------

fn extract_searchable_text(event: &AgentLogEvent) -> String {
    let mut parts: Vec<String> = Vec::new();

    match &event.kind {
        LogEventKind::SessionStarted {
            command,
            model,
            cwd,
            ..
        } => {
            parts.push(command.clone());
            if let Some(m) = model {
                parts.push(m.clone());
            }
            if let Some(c) = cwd {
                parts.push(c.clone());
            }
        }
        LogEventKind::UserMessage { role, content, .. } => {
            parts.push(role.clone());
            parts.push(content.clone());
        }
        LogEventKind::AssistantMessage { content, .. } => {
            parts.push(content.clone());
        }
        LogEventKind::Reasoning { content, .. } => {
            parts.push(content.clone());
        }
        LogEventKind::ToolCall {
            tool_name, input, ..
        } => {
            parts.push(tool_name.clone());
            if let Some(v) = input {
                parts.push(v.to_string());
            }
        }
        LogEventKind::ToolResult {
            tool_name,
            output,
            error,
            data,
            ..
        } => {
            if let Some(n) = tool_name {
                parts.push(n.clone());
            }
            if let Some(o) = output {
                parts.push(o.clone());
            }
            if let Some(e) = error {
                parts.push(e.clone());
            }
            if let Some(d) = data {
                parts.push(d.to_string());
            }
        }
        LogEventKind::Permission {
            tool_name,
            description,
            ..
        } => {
            parts.push(tool_name.clone());
            parts.push(description.clone());
        }
        LogEventKind::ProviderStatus { message, .. } => {
            parts.push(message.clone());
        }
        LogEventKind::Stderr { message } => {
            parts.push(message.clone());
        }
        LogEventKind::ParseWarning { message, raw } => {
            parts.push(message.clone());
            if let Some(r) = raw {
                parts.push(r.clone());
            }
        }
        LogEventKind::SessionEnded { error, .. } => {
            if let Some(e) = error {
                parts.push(e.clone());
            }
        }
        LogEventKind::SessionCleared { .. } => {}
    }

    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Snippet builder
// ---------------------------------------------------------------------------

fn make_snippet(text: &str, matcher: &TextMatcher, max_len: usize) -> String {
    let offset = matcher.find_offset(text).unwrap_or(0);

    let start = offset.saturating_sub(max_len / 4);
    let end = (start + max_len).min(text.len());

    // Clamp to char boundaries.
    let start = text
        .char_indices()
        .map(|(i, _)| i)
        .rfind(|&i| i <= start)
        .unwrap_or(0);
    let end = text
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= end)
        .unwrap_or(text.len());

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("[...] ");
    }
    snippet.push_str(&text[start..end]);
    if end < text.len() {
        snippet.push_str(" [...]");
    }
    snippet
}

// ---------------------------------------------------------------------------
// Metadata pre-filter
// ---------------------------------------------------------------------------

fn session_matches_query(entry: &SessionLogIndexEntry, query: &SearchQuery) -> bool {
    // Provider filter
    if let Some(ref p) = query.provider
        && !entry.provider.eq_ignore_ascii_case(p)
    {
        return false;
    }

    // Session ID prefix filter
    if let Some(ref sid) = query.session_id
        && !entry.wrapper_session_id.starts_with(sid.as_str())
    {
        return false;
    }

    // Date range: skip sessions that definitely ended before `from`
    if let Some(from) = query.from
        && let Some(ref ended) = entry.ended_at
        && let Ok(ended_dt) = DateTime::parse_from_rfc3339(ended)
        && ended_dt.with_timezone(&Utc) < from
    {
        return false;
    }

    // Date range: skip sessions that started after `to`
    if let Some(to) = query.to
        && let Ok(started_dt) = DateTime::parse_from_rfc3339(&entry.started_at)
        && started_dt.with_timezone(&Utc) > to
    {
        return false;
    }

    true
}

// ---------------------------------------------------------------------------
// Event filter
// ---------------------------------------------------------------------------

fn event_matches_query(event: &AgentLogEvent, query: &SearchQuery, matcher: &TextMatcher) -> bool {
    // Provider filter at event level
    if let Some(ref p) = query.provider
        && !event.provider.eq_ignore_ascii_case(p)
    {
        return false;
    }

    // Date range filters
    if (query.from.is_some() || query.to.is_some())
        && let Ok(event_dt) = DateTime::parse_from_rfc3339(&event.ts)
    {
        let event_utc = event_dt.with_timezone(&Utc);
        if let Some(from) = query.from
            && event_utc < from
        {
            return false;
        }
        if let Some(to) = query.to
            && event_utc > to
        {
            return false;
        }
    }

    // Tool kind / tool name / role filters
    let has_tool_filter = query.tool.is_some() || query.tool_kind.is_some();
    let has_role_filter = query.role.is_some();

    if has_tool_filter {
        match &event.kind {
            LogEventKind::ToolCall {
                tool_name,
                tool_kind,
                ..
            } => {
                if let Some(ref t) = query.tool
                    && !tool_name.to_lowercase().contains(&t.to_lowercase())
                {
                    return false;
                }
                if let Some(ref tk) = query.tool_kind {
                    let kind = tool_kind.unwrap_or_else(|| ToolKind::infer(tool_name));
                    if kind != *tk {
                        return false;
                    }
                }
            }
            LogEventKind::ToolResult {
                tool_name,
                tool_kind,
                ..
            } => {
                if let Some(ref t) = query.tool {
                    let name = tool_name.as_deref().unwrap_or("");
                    if !name.to_lowercase().contains(&t.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref tk) = query.tool_kind {
                    let kind = tool_kind.unwrap_or_else(|| {
                        tool_name
                            .as_deref()
                            .map(ToolKind::infer)
                            .unwrap_or(ToolKind::Other)
                    });
                    if kind != *tk {
                        return false;
                    }
                }
            }
            // Non-tool events are excluded when a tool filter is active
            _ => return false,
        }
    }

    if has_role_filter {
        match &event.kind {
            LogEventKind::UserMessage { role, .. } => {
                if let Some(ref r) = query.role
                    && !role.eq_ignore_ascii_case(r)
                {
                    return false;
                }
            }
            // Non-message events are excluded when a role filter is active
            // (unless combined with a tool filter, which we already handled above)
            _ if !has_tool_filter => return false,
            _ => {}
        }
    }

    // Text filter
    if matcher.has_filter() {
        let text = extract_searchable_text(event);
        if !matcher.is_match(&text) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// JSONL scanner
// ---------------------------------------------------------------------------

struct ScanResult {
    events_scanned: usize,
    matching_events: Vec<AgentLogEvent>,
}

fn scan_session(log_path: &Path, query: &SearchQuery, matcher: &TextMatcher) -> Result<ScanResult> {
    let file = std::fs::File::open(log_path)
        .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;
    let reader = BufReader::new(file);

    let mut result = ScanResult {
        events_scanned: 0,
        matching_events: Vec::new(),
    };

    for line in reader.lines() {
        let line =
            line.with_context(|| format!("Failed to read line in {}", log_path.display()))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let event: AgentLogEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                log::debug!(
                    "Skipping malformed JSONL line in {}: {}",
                    log_path.display(),
                    e
                );
                continue;
            }
        };

        result.events_scanned += 1;

        if event_matches_query(&event, query, matcher) {
            result.matching_events.push(event);
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Session discovery
// ---------------------------------------------------------------------------

fn collect_candidate_sessions(
    query: &SearchQuery,
    zag_home: &Path,
    cwd: &Path,
) -> Result<Vec<(SessionLogIndexEntry, PathBuf)>> {
    let projects_dir = zag_home.join("projects");
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    // If tag filter is set, collect matching session IDs from session stores.
    let tag_session_ids: Option<std::collections::HashSet<String>> = if query.tag.is_some() {
        let store = if query.global {
            crate::session::SessionStore::load_all().unwrap_or_default()
        } else {
            crate::session::SessionStore::load(Some(&cwd.to_string_lossy())).unwrap_or_default()
        };
        let tag = query.tag.as_deref().unwrap();
        let matching = store.find_by_tag(tag);
        Some(matching.into_iter().map(|e| e.session_id.clone()).collect())
    } else {
        None
    };

    let cwd_str = cwd.to_string_lossy().to_string();
    let mut candidates: Vec<(SessionLogIndexEntry, PathBuf)> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let read_dir = std::fs::read_dir(&projects_dir)
        .with_context(|| format!("Failed to read {}", projects_dir.display()))?;

    for entry in read_dir {
        let project_dir = match entry {
            Ok(e) => e.path(),
            Err(_) => continue,
        };
        if !project_dir.is_dir() {
            continue;
        }

        let index_path = project_dir.join("logs").join("index.json");
        if !index_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&index_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to read index {}: {}", index_path.display(), e);
                continue;
            }
        };

        let index: SessionLogIndex = match serde_json::from_str(&content) {
            Ok(i) => i,
            Err(e) => {
                log::warn!("Malformed index {}: {}", index_path.display(), e);
                continue;
            }
        };

        for session_entry in index.sessions {
            // Scope filter: in non-global mode, only include sessions whose workspace_path
            // is within the current directory tree.
            if !query.global {
                let in_scope = match &session_entry.workspace_path {
                    Some(wp) => {
                        // Match if workspace is the cwd or a subdirectory of cwd
                        wp == &cwd_str
                            || wp.starts_with(&format!("{}/", cwd_str))
                            || wp.starts_with(&format!("{}\\", cwd_str))
                    }
                    None => false,
                };
                if !in_scope {
                    continue;
                }
            }

            // Metadata pre-filter (provider, session ID, dates)
            if !session_matches_query(&session_entry, query) {
                continue;
            }

            // Tag filter: only include sessions matching the tag
            if let Some(ref allowed) = tag_session_ids {
                if !allowed.contains(&session_entry.wrapper_session_id) {
                    continue;
                }
            }

            // Deduplicate by session ID
            if !seen_ids.insert(session_entry.wrapper_session_id.clone()) {
                continue;
            }

            let log_path = PathBuf::from(&session_entry.log_path);
            candidates.push((session_entry, log_path));
        }
    }

    // Sort by started_at so results are in chronological order
    candidates.sort_by(|a, b| a.0.started_at.cmp(&b.0.started_at));

    Ok(candidates)
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Search through session logs matching the given query.
pub fn search(query: &SearchQuery, zag_home: &Path, cwd: &Path) -> Result<SearchResults> {
    let matcher = TextMatcher::build(query)?;

    let candidates = collect_candidate_sessions(query, zag_home, cwd)?;

    let mut results = SearchResults::default();

    'outer: for (entry, log_path) in candidates {
        results.total_sessions_scanned += 1;

        if !log_path.exists() {
            results.total_files_missing += 1;
            log::debug!("Log file missing: {}", log_path.display());
            continue;
        }

        let scan = match scan_session(&log_path, query, &matcher) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to scan {}: {}", log_path.display(), e);
                continue;
            }
        };

        results.total_events_scanned += scan.events_scanned;

        for event in scan.matching_events {
            let text = extract_searchable_text(&event);
            let snippet = make_snippet(&text, &matcher, 200);

            results.matches.push(SearchMatch {
                session_id: entry.wrapper_session_id.clone(),
                provider: entry.provider.clone(),
                started_at: entry.started_at.clone(),
                ended_at: entry.ended_at.clone(),
                workspace_path: entry.workspace_path.clone(),
                command: entry.command.clone(),
                event,
                snippet,
            });

            if let Some(limit) = query.limit
                && results.matches.len() >= limit
            {
                break 'outer;
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
