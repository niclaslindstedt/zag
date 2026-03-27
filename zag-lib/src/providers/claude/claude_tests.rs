use super::Claude;
use crate::sandbox::SandboxConfig;

#[test]
fn test_build_run_args_non_interactive() {
    let mut claude = Claude::new();
    claude.model = "opus".to_string();

    let fmt = Some("json".to_string());
    let args = claude.build_run_args(false, Some("hello"), &fmt);

    assert!(args.contains(&"--print".to_string()));
    assert!(args.contains(&"--verbose".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive() {
    let claude = Claude::new();
    let args = claude.build_run_args(true, Some("hello"), &None);

    assert!(!args.contains(&"--print".to_string()));
    assert!(!args.contains(&"--verbose".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_skip_permissions() {
    let mut claude = Claude::new();
    claude.skip_permissions = true;

    let args = claude.build_run_args(true, None, &None);
    assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_build_run_args_sandbox_skips_permissions() {
    let mut claude = Claude::new();
    claude.skip_permissions = true;
    claude.sandbox = Some(SandboxConfig {
        name: "test".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let args = claude.build_run_args(true, None, &None);
    assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_build_run_args_with_system_prompt() {
    let mut claude = Claude::new();
    claude.system_prompt = "You are helpful".to_string();

    let args = claude.build_run_args(true, None, &None);
    assert!(args.contains(&"--append-system-prompt".to_string()));
    assert!(args.contains(&"You are helpful".to_string()));
}

#[test]
fn test_build_run_args_input_format_only_non_interactive() {
    let mut claude = Claude::new();
    claude.input_format = Some("stream-json".to_string());

    let interactive_args = claude.build_run_args(true, None, &None);
    assert!(!interactive_args.contains(&"--input-format".to_string()));

    let non_interactive_args = claude.build_run_args(false, Some("hello"), &None);
    assert!(non_interactive_args.contains(&"--input-format".to_string()));
    assert!(non_interactive_args.contains(&"stream-json".to_string()));
}

#[test]
fn test_build_run_args_replay_user_messages_only_non_interactive() {
    let mut claude = Claude::new();
    claude.replay_user_messages = true;

    let interactive_args = claude.build_run_args(true, None, &None);
    assert!(!interactive_args.contains(&"--replay-user-messages".to_string()));

    let non_interactive_args = claude.build_run_args(false, Some("hello"), &None);
    assert!(non_interactive_args.contains(&"--replay-user-messages".to_string()));
}

#[test]
fn test_build_run_args_include_partial_messages_only_non_interactive() {
    let mut claude = Claude::new();
    claude.include_partial_messages = true;

    let interactive_args = claude.build_run_args(true, None, &None);
    assert!(!interactive_args.contains(&"--include-partial-messages".to_string()));

    let non_interactive_args = claude.build_run_args(false, Some("hello"), &None);
    assert!(non_interactive_args.contains(&"--include-partial-messages".to_string()));
}

#[test]
fn test_build_resume_args() {
    let mut claude = Claude::new();
    claude.model = "sonnet".to_string();

    let args = claude.build_resume_args(Some("session-123"));
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"session-123".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"sonnet".to_string()));
}

#[test]
fn test_build_resume_args_continue() {
    let claude = Claude::new();
    let args = claude.build_resume_args(None);
    assert!(args.contains(&"--continue".to_string()));
    assert!(!args.contains(&"--resume".to_string()));
}

#[test]
fn test_build_resume_args_sandbox_skips_permissions() {
    let mut claude = Claude::new();
    claude.skip_permissions = true;
    claude.sandbox = Some(SandboxConfig {
        name: "test".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let args = claude.build_resume_args(Some("sid"));
    assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut claude = Claude::new();
    claude.root = Some("/project".to_string());

    let cmd = claude.make_command(vec!["--print".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "claude");
    assert_eq!(
        cmd.as_std().get_current_dir().unwrap().to_str().unwrap(),
        "/project"
    );
}

#[test]
fn test_make_command_with_sandbox() {
    let mut claude = Claude::new();
    claude.sandbox = Some(SandboxConfig {
        name: "sandbox-abc".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    });

    let cmd = claude.make_command(vec!["--print".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"sandbox-abc"));
    assert!(args.contains(&"--print"));
    assert!(args.contains(&"hello"));
}
