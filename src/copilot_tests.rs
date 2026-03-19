use super::Copilot;
use crate::sandbox::SandboxConfig;

#[test]
fn test_build_run_args_non_interactive() {
    let mut copilot = Copilot::new();
    copilot.model = "claude-sonnet-4.5".to_string();

    let args = copilot.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--allow-all-tools".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"claude-sonnet-4.5".to_string()));
    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive_with_prompt() {
    let copilot = Copilot::new();
    let args = copilot.build_run_args(true, Some("hello"));
    assert!(!args.contains(&"--allow-all-tools".to_string()));
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive_no_prompt() {
    let copilot = Copilot::new();
    let args = copilot.build_run_args(true, None);
    assert!(!args.contains(&"-i".to_string()));
    assert!(!args.contains(&"-p".to_string()));
}

#[test]
fn test_build_run_args_skip_permissions() {
    let mut copilot = Copilot::new();
    copilot.skip_permissions = true;

    let args = copilot.build_run_args(true, None);
    assert!(args.contains(&"--allow-all-tools".to_string()));
}

#[test]
fn test_build_run_args_add_dirs() {
    let mut copilot = Copilot::new();
    copilot.add_dirs = vec!["/extra".to_string()];

    let args = copilot.build_run_args(true, None);
    assert!(args.contains(&"--add-dir".to_string()));
    assert!(args.contains(&"/extra".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut copilot = Copilot::new();
    copilot.root = Some("/project".to_string());

    let cmd = copilot.make_command(vec!["-p".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "copilot");
    assert_eq!(
        cmd.as_std().get_current_dir().unwrap().to_str().unwrap(),
        "/project"
    );
}

#[test]
fn test_make_command_with_sandbox() {
    let mut copilot = Copilot::new();
    copilot.sandbox = Some(SandboxConfig {
        name: "sandbox-cp".to_string(),
        template: "docker/sandbox-templates:copilot".to_string(),
        workspace: "/workspace".to_string(),
    });

    let cmd = copilot.make_command(vec!["-p".to_string(), "hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"sandbox-cp"));
    assert!(args.contains(&"-p"));
    assert!(args.contains(&"hello"));
}
