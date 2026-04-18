use super::*;

#[test]
fn resolve_pipe_sessions_with_explicit_ids() {
    let ids = vec!["abc".to_string(), "def".to_string()];
    let result = resolve_pipe_sessions(&ids, None, None).unwrap();
    assert_eq!(result, ids);
}

#[test]
fn resolve_pipe_sessions_empty_errors() {
    let result = resolve_pipe_sessions(&[], None, None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No sessions specified")
    );
}

#[test]
fn build_context_single_session_no_index() {
    // Can't resolve a nonexistent session — returns None from extract
    let result = build_context(&["nonexistent".to_string()], None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No results available")
    );
}

#[test]
fn pipe_params_exposes_full_session_setup_surface() {
    // Construct a PipeParams with every field populated so any future shape
    // drift surfaces at compile time. Mirrors the `zag run`/`zag exec` surface.
    let params = PipeParams {
        session_ids: vec!["sid-1".to_string()],
        tag: Some("batch".to_string()),
        prompt: "summarize".to_string(),
        provider: Some("claude".to_string()),
        model: Some("sonnet".to_string()),
        root: Some("/tmp/workspace".to_string()),
        auto_approve: true,
        system_prompt: Some("be concise".to_string()),
        add_dirs: vec!["../shared".to_string()],
        size: Some("2b".to_string()),
        max_turns: Some(4),
        output: Some("json".to_string()),
        json: true,
        quiet: true,
        metadata: SessionMetadata {
            name: Some("pipe-followup".to_string()),
            description: Some("combines prior analyses".to_string()),
            tags: vec!["followup".to_string(), "pipe".to_string()],
        },
        timeout: Some("30s".to_string()),
        env_vars: vec![("DEBUG".to_string(), "1".to_string())],
        files: vec!["notes.md".to_string()],
        worktree: Some(Some("wt-pipe".to_string())),
        sandbox: Some(None),
        context: Some("prev-session-id".to_string()),
        mcp_config: Some(r#"{"mcpServers":{}}"#.to_string()),
    };

    assert_eq!(params.metadata.name.as_deref(), Some("pipe-followup"));
    assert_eq!(params.metadata.tags.len(), 2);
    assert_eq!(params.timeout.as_deref(), Some("30s"));
    assert_eq!(params.env_vars.len(), 1);
    assert_eq!(params.files, vec!["notes.md".to_string()]);
    assert!(matches!(params.worktree, Some(Some(_))));
    assert!(matches!(params.sandbox, Some(None)));
    assert_eq!(params.context.as_deref(), Some("prev-session-id"));
    assert!(params.mcp_config.is_some());
}
