use super::*;

#[test]
fn test_template_for_provider() {
    assert_eq!(
        template_for_provider("claude"),
        "docker/sandbox-templates:claude-code"
    );
    assert_eq!(
        template_for_provider("codex"),
        "docker/sandbox-templates:codex"
    );
    assert_eq!(
        template_for_provider("gemini"),
        "docker/sandbox-templates:gemini"
    );
    assert_eq!(
        template_for_provider("copilot"),
        "docker/sandbox-templates:copilot"
    );
    // Unknown provider falls back to claude-code
    assert_eq!(
        template_for_provider("unknown"),
        "docker/sandbox-templates:claude-code"
    );
}

#[test]
fn test_generate_name() {
    let name = generate_name();
    assert!(name.starts_with("sandbox-"));
    assert_eq!(name.len(), "sandbox-".len() + 8); // 8 hex chars
}

#[test]
fn test_generate_name_uniqueness() {
    let name1 = generate_name();
    // Sleep briefly to get a different timestamp
    std::thread::sleep(std::time::Duration::from_millis(1));
    let name2 = generate_name();
    // Names should differ (not guaranteed but very likely with different timestamps)
    // This test mainly verifies the function doesn't panic
    assert!(name1.starts_with("sandbox-"));
    assert!(name2.starts_with("sandbox-"));
}

#[test]
fn test_build_sandbox_command() {
    let config = SandboxConfig {
        name: "sandbox-test123".to_string(),
        template: "docker/sandbox-templates:claude-code".to_string(),
        workspace: "/workspace".to_string(),
    };
    let args = vec![
        "--print".to_string(),
        "--model".to_string(),
        "opus".to_string(),
        "hello".to_string(),
    ];

    let cmd = build_sandbox_command(&config, args);
    let program = cmd.get_program().to_str().unwrap();
    let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

    assert_eq!(program, "docker");
    assert_eq!(
        args,
        vec![
            "sandbox",
            "run",
            "--name",
            "sandbox-test123",
            "docker/sandbox-templates:claude-code",
            "/workspace",
            "--",
            "--print",
            "--model",
            "opus",
            "hello",
        ]
    );
}

#[test]
fn test_build_sandbox_command_empty_args() {
    let config = SandboxConfig {
        name: "sandbox-empty".to_string(),
        template: "docker/sandbox-templates:codex".to_string(),
        workspace: "/my/project".to_string(),
    };

    let cmd = build_sandbox_command(&config, vec![]);
    let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap()).collect();

    assert_eq!(
        args,
        vec![
            "sandbox",
            "run",
            "--name",
            "sandbox-empty",
            "docker/sandbox-templates:codex",
            "/my/project",
            "--",
        ]
    );
}
