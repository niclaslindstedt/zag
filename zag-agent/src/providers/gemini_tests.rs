use super::Gemini;
use crate::agent::Agent;
use crate::sandbox::SandboxConfig;

#[test]
fn test_build_run_args_non_interactive() {
    let mut gemini = Gemini::new();
    gemini.model = "gemini-2.5-pro".to_string();
    gemini.output_format = Some("json".to_string());

    let args = gemini.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gemini-2.5-pro".to_string()));
    assert!(args.contains(&"--output-format".to_string()));
    assert!(args.contains(&"json".to_string()));
    assert!(args.contains(&"hello".to_string()));
}

#[test]
fn test_build_run_args_interactive_no_output_format() {
    let mut gemini = Gemini::new();
    gemini.output_format = Some("json".to_string());

    let args = gemini.build_run_args(true, Some("hello"));
    assert!(!args.contains(&"--output-format".to_string()));
}

#[test]
fn test_build_run_args_auto_model_skipped() {
    let gemini = Gemini::new(); // default model is "auto"
    let args = gemini.build_run_args(true, None);
    assert!(!args.contains(&"--model".to_string()));
}

#[test]
fn test_build_run_args_skip_permissions() {
    let mut gemini = Gemini::new();
    gemini.skip_permissions = true;

    let args = gemini.build_run_args(true, None);
    assert!(args.contains(&"--approval-mode".to_string()));
    assert!(args.contains(&"yolo".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut gemini = Gemini::new();
    gemini.root = Some("/project".to_string());

    let cmd = gemini.make_command(vec!["hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "gemini");
    assert_eq!(
        cmd.as_std().get_current_dir().unwrap().to_str().unwrap(),
        "/project"
    );
}

#[test]
fn test_make_command_with_sandbox() {
    let mut gemini = Gemini::new();
    gemini.sandbox = Some(SandboxConfig {
        name: "sandbox-gem".to_string(),
        template: "docker/sandbox-templates:gemini".to_string(),
        workspace: "/workspace".to_string(),
    });

    let cmd = gemini.make_command(vec!["hello".to_string()]);
    assert_eq!(cmd.as_std().get_program().to_str().unwrap(), "docker");
    let args: Vec<&str> = cmd
        .as_std()
        .get_args()
        .map(|a| a.to_str().unwrap())
        .collect();
    assert!(args.contains(&"sandbox"));
    assert!(args.contains(&"run"));
    assert!(args.contains(&"sandbox-gem"));
    assert!(args.contains(&"hello"));
}

#[test]
fn test_build_run_args_max_turns() {
    let mut gemini = Gemini::new();
    gemini.max_turns = Some(10);

    let args = gemini.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--max-turns".to_string()));
    assert!(args.contains(&"10".to_string()));
}

#[test]
fn test_build_run_args_no_max_turns_by_default() {
    let gemini = Gemini::new();
    let args = gemini.build_run_args(false, Some("hello"));
    assert!(!args.contains(&"--max-turns".to_string()));
}

#[test]
fn test_available_models_includes_3_1() {
    let models = Gemini::available_models();
    assert!(models.contains(&"gemini-3.1-pro-preview"));
    assert!(models.contains(&"gemini-3.1-flash-lite-preview"));
    assert!(models.contains(&"gemini-3-pro-preview"));
    assert!(models.contains(&"gemini-2.5-pro"));
}

#[test]
fn test_model_for_size_uses_latest() {
    use crate::agent::ModelSize;
    assert_eq!(
        Gemini::model_for_size(ModelSize::Large),
        "gemini-3.1-pro-preview"
    );
    assert_eq!(
        Gemini::model_for_size(ModelSize::Medium),
        "gemini-2.5-flash"
    );
    assert_eq!(
        Gemini::model_for_size(ModelSize::Small),
        "gemini-3.1-flash-lite-preview"
    );
}

#[test]
fn test_build_run_args_with_3_1_model() {
    let mut gemini = Gemini::new();
    gemini.model = "gemini-3.1-pro-preview".to_string();

    let args = gemini.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gemini-3.1-pro-preview".to_string()));
}
