use super::*;

#[test]
fn test_model_tag() {
    let ollama = Ollama::new();
    assert_eq!(ollama.model_tag(), "qwen3.5:9b");
}

#[test]
fn test_model_tag_custom() {
    let mut ollama = Ollama::new();
    ollama.model = "llama3".to_string();
    ollama.size = "70b".to_string();
    assert_eq!(ollama.model_tag(), "llama3:70b");
}

#[test]
fn test_build_run_args_interactive() {
    let ollama = Ollama::new();
    let args = ollama.build_run_args(true, None);
    assert_eq!(args[0], "run");
    assert!(args.contains(&"qwen3.5:9b".to_string()));
    assert!(!args.contains(&"--nowordwrap".to_string()));
    assert!(args.contains(&"--hidethinking".to_string()));
}

#[test]
fn test_build_run_args_non_interactive() {
    let ollama = Ollama::new();
    let args = ollama.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--nowordwrap".to_string()));
    assert!(args.contains(&"--hidethinking".to_string()));
    assert!(args.contains(&"qwen3.5:9b".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_with_system_prompt_no_user_prompt() {
    let mut ollama = Ollama::new();
    ollama.system_prompt = "You are helpful".to_string();
    let args = ollama.build_run_args(true, None);
    // --system is not a valid ollama run flag; system prompt is prepended to user prompt
    assert!(!args.contains(&"--system".to_string()));
    assert!(args.contains(&"You are helpful".to_string()));
}

#[test]
fn test_build_run_args_with_system_prompt_and_user_prompt() {
    let mut ollama = Ollama::new();
    ollama.system_prompt = "Be concise".to_string();
    let args = ollama.build_run_args(false, Some("say hello"));
    assert!(!args.contains(&"--system".to_string()));
    // system prompt and user prompt merged
    let last = args.last().unwrap();
    assert!(last.contains("Be concise"));
    assert!(last.contains("say hello"));
}

#[test]
fn test_build_run_args_json_format() {
    let mut ollama = Ollama::new();
    ollama.output_format = Some("json".to_string());
    let args = ollama.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--format".to_string()));
    assert!(args.contains(&"json".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut ollama = Ollama::new();
    ollama.root = Some("/project".to_string());
    let cmd = ollama.make_command(vec!["run".to_string(), "qwen3.5:9b".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "ollama");
    assert_eq!(
        cmd.as_std().get_current_dir().unwrap().to_str().unwrap(),
        "/project"
    );
}

#[test]
fn test_make_command_with_sandbox() {
    let mut ollama = Ollama::new();
    ollama.sandbox = Some(SandboxConfig {
        name: "sandbox-oll".to_string(),
        template: "shell".to_string(),
        workspace: "/workspace".to_string(),
    });
    let cmd = ollama.make_command(vec![
        "run".to_string(),
        "qwen3.5:9b".to_string(),
        "hello".to_string(),
    ]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"--name"));
    assert!(args.contains(&"sandbox-oll"));
    assert!(args.contains(&"shell"));
    assert!(args.contains(&"/workspace"));
    assert!(args.contains(&"-c"));
    // The shell command should contain ollama run
    let shell_cmd = args.last().unwrap();
    assert!(shell_cmd.contains("ollama"));
    assert!(shell_cmd.contains("qwen3.5:9b"));
}

#[test]
fn test_size_for_model_size() {
    assert_eq!(Ollama::size_for_model_size(ModelSize::Small), "2b");
    assert_eq!(Ollama::size_for_model_size(ModelSize::Medium), "9b");
    assert_eq!(Ollama::size_for_model_size(ModelSize::Large), "35b");
}

#[test]
fn test_set_size() {
    let mut ollama = Ollama::new();
    ollama.set_size("27b".to_string());
    assert_eq!(ollama.model_tag(), "qwen3.5:27b");
}

#[test]
fn test_shell_escape_simple() {
    assert_eq!(shell_escape("hello"), "hello");
}

#[test]
fn test_shell_escape_spaces() {
    assert_eq!(shell_escape("hello world"), "'hello world'");
}

#[test]
fn test_shell_escape_quotes() {
    assert_eq!(shell_escape("it's"), "'it'\\''s'");
}
