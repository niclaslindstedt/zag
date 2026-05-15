use super::Gemini;
use crate::agent::Agent;
use crate::sandbox::SandboxConfig;

#[test]
fn test_build_run_args_non_interactive() {
    let mut gemini = Gemini::new();
    gemini.common.model = "gemini-2.5-pro".to_string();
    gemini.common.output_format = Some("json".to_string());

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
    gemini.common.output_format = Some("json".to_string());

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
    gemini.common.skip_permissions = true;

    let args = gemini.build_run_args(true, None);
    assert!(args.contains(&"--approval-mode".to_string()));
    assert!(args.contains(&"yolo".to_string()));
}

#[test]
fn test_make_command_without_sandbox() {
    let mut gemini = Gemini::new();
    gemini.common.root = Some("/project".to_string());

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
    gemini.common.sandbox = Some(SandboxConfig {
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
fn test_build_run_args_max_turns_not_passed() {
    let mut gemini = Gemini::new();
    gemini.common.max_turns = Some(10);

    // Gemini CLI does not support --max-turns as a CLI flag
    let args = gemini.build_run_args(false, Some("hello"));
    assert!(!args.contains(&"--max-turns".to_string()));
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
    gemini.common.model = "gemini-3.1-pro-preview".to_string();

    let args = gemini.build_run_args(false, Some("hello"));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gemini-3.1-pro-preview".to_string()));
}

/// A prompt starting with `---` must not be misread as an unknown
/// long option by the gemini CLI. `build_run_args` terminates option
/// parsing with `--` immediately before the positional prompt.
#[test]
fn test_build_run_args_prompt_is_guarded_by_double_dash() {
    let gemini = Gemini::new();
    let args = gemini.build_run_args(false, Some("--- context ---"));

    let dd_idx = args
        .iter()
        .position(|a| a == "--")
        .expect("expected `--` separator before the prompt");
    let prompt_idx = args
        .iter()
        .position(|a| a == "--- context ---")
        .expect("expected prompt to be present");
    assert_eq!(
        dd_idx + 1,
        prompt_idx,
        "`--` must be the token immediately preceding the prompt"
    );
}

#[test]
fn test_build_run_args_no_prompt_has_no_double_dash_separator() {
    let gemini = Gemini::new();
    let args = gemini.build_run_args(true, None);
    assert!(!args.contains(&"--".to_string()));
}

#[test]
fn test_build_resume_args_basic() {
    let mut gemini = Gemini::new();
    gemini.common.model = "gemini-2.5-pro".to_string();

    let args = gemini.build_resume_args("session-xyz", "Continue");
    // --resume <id> appears
    let r_idx = args
        .iter()
        .position(|a| a == "--resume")
        .expect("--resume present");
    assert_eq!(args[r_idx + 1], "session-xyz");
    // model flag passed through
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"gemini-2.5-pro".to_string()));
    // -- precedes the prompt so a leading-dash prompt isn't misread as a flag
    let dash_idx = args
        .iter()
        .position(|a| a == "--")
        .expect("`--` separator present");
    assert_eq!(args[dash_idx + 1], "Continue");
    // The separator and prompt come last
    assert_eq!(dash_idx, args.len() - 2);
}

#[test]
fn test_build_resume_args_skips_auto_model() {
    let gemini = Gemini::new(); // default model is "auto"
    let args = gemini.build_resume_args("sid", "go");
    assert!(!args.contains(&"--model".to_string()));
}

#[test]
fn test_build_resume_args_skip_permissions_yolo() {
    let mut gemini = Gemini::new();
    gemini.common.skip_permissions = true;

    let args = gemini.build_resume_args("sid", "go");
    assert!(args.windows(2).any(|w| w == ["--approval-mode", "yolo"]));
}

#[test]
fn test_build_resume_args_includes_dirs_and_format() {
    let mut gemini = Gemini::new();
    gemini.common.add_dirs = vec!["/extra".to_string()];
    gemini.common.output_format = Some("json".to_string());

    let args = gemini.build_resume_args("sid", "go");
    assert!(
        args.windows(2)
            .any(|w| w == ["--include-directories", "/extra"])
    );
    assert!(args.windows(2).any(|w| w == ["--output-format", "json"]));
}

#[test]
fn test_build_resume_args_quotes_dash_leading_prompt_via_separator() {
    let gemini = Gemini::new();
    let args = gemini.build_resume_args("sid", "--evil-prompt");
    // The prompt comes right after `--` so the gemini CLI treats it as positional.
    let dash_idx = args.iter().position(|a| a == "--").unwrap();
    assert_eq!(args[dash_idx + 1], "--evil-prompt");
}
