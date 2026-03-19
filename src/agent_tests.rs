use super::*;
use crate::claude::Claude;
use crate::codex::Codex;
use crate::copilot::Copilot;
use crate::gemini::Gemini;

#[test]
fn test_model_size_from_str() {
    assert_eq!("small".parse::<ModelSize>(), Ok(ModelSize::Small));
    assert_eq!("s".parse::<ModelSize>(), Ok(ModelSize::Small));
    assert_eq!("SMALL".parse::<ModelSize>(), Ok(ModelSize::Small));
    assert_eq!("medium".parse::<ModelSize>(), Ok(ModelSize::Medium));
    assert_eq!("m".parse::<ModelSize>(), Ok(ModelSize::Medium));
    assert_eq!("default".parse::<ModelSize>(), Ok(ModelSize::Medium));
    assert_eq!("large".parse::<ModelSize>(), Ok(ModelSize::Large));
    assert_eq!("l".parse::<ModelSize>(), Ok(ModelSize::Large));
    assert_eq!("max".parse::<ModelSize>(), Ok(ModelSize::Large));
    assert_eq!("opus".parse::<ModelSize>(), Err(()));
    assert_eq!("gpt-5".parse::<ModelSize>(), Err(()));
    assert_eq!("".parse::<ModelSize>(), Err(()));
}

#[test]
fn test_claude_resolve_model() {
    assert_eq!(Claude::resolve_model("small"), "haiku");
    assert_eq!(Claude::resolve_model("medium"), "sonnet");
    assert_eq!(Claude::resolve_model("large"), "opus");
    assert_eq!(Claude::resolve_model("sonnet"), "sonnet"); // passthrough
}

#[test]
fn test_codex_resolve_model() {
    assert_eq!(Codex::resolve_model("small"), "gpt-5.1-codex-mini");
    assert_eq!(Codex::resolve_model("medium"), "gpt-5.2-codex");
    assert_eq!(Codex::resolve_model("large"), "gpt-5.1-codex-max");
    assert_eq!(Codex::resolve_model("gpt-5.2"), "gpt-5.2"); // passthrough
}

#[test]
fn test_gemini_resolve_model() {
    assert_eq!(Gemini::resolve_model("small"), "gemini-2.5-flash-lite");
    assert_eq!(Gemini::resolve_model("medium"), "gemini-2.5-flash");
    assert_eq!(Gemini::resolve_model("large"), "gemini-2.5-pro");
    assert_eq!(Gemini::resolve_model("auto"), "auto"); // passthrough
}

#[test]
fn test_copilot_resolve_model() {
    assert_eq!(Copilot::resolve_model("small"), "claude-haiku-4.5");
    assert_eq!(Copilot::resolve_model("medium"), "claude-sonnet-4.5");
    assert_eq!(Copilot::resolve_model("large"), "claude-opus-4.5");
    assert_eq!(Copilot::resolve_model("gpt-5"), "gpt-5"); // passthrough
}

#[test]
fn test_short_aliases() {
    assert_eq!(Claude::resolve_model("s"), "haiku");
    assert_eq!(Claude::resolve_model("m"), "sonnet");
    assert_eq!(Claude::resolve_model("l"), "opus");
    assert_eq!(Codex::resolve_model("max"), "gpt-5.1-codex-max");
}

#[test]
fn test_validate_model_valid() {
    assert!(Claude::validate_model("sonnet", "Claude").is_ok());
    assert!(Claude::validate_model("opus", "Claude").is_ok());
    assert!(Claude::validate_model("haiku", "Claude").is_ok());
}

#[test]
fn test_validate_model_invalid() {
    let result = Claude::validate_model("gpt-5", "Claude");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid model"));
    assert!(err.contains("Claude"));
    // Error should list size mappings
    assert!(err.contains("small"));
    assert!(err.contains("medium"));
    assert!(err.contains("large"));
}

#[test]
fn test_validate_model_all_agents() {
    assert!(Codex::validate_model("gpt-5.2-codex", "Codex").is_ok());
    assert!(Codex::validate_model("invalid", "Codex").is_err());

    assert!(Gemini::validate_model("auto", "Gemini").is_ok());
    assert!(Gemini::validate_model("invalid", "Gemini").is_err());

    assert!(Copilot::validate_model("claude-sonnet-4.5", "Copilot").is_ok());
    assert!(Copilot::validate_model("invalid", "Copilot").is_err());
}

#[test]
fn test_default_models() {
    assert_eq!(Claude::default_model(), "opus");
    assert_eq!(Codex::default_model(), "gpt-5.2-codex");
    assert_eq!(Gemini::default_model(), "auto");
    assert_eq!(Copilot::default_model(), "claude-sonnet-4.5");
}

#[test]
fn test_available_models() {
    assert!(Claude::available_models().contains(&"sonnet"));
    assert!(Claude::available_models().contains(&"opus"));
    assert!(Claude::available_models().contains(&"haiku"));

    assert!(Codex::available_models().contains(&"gpt-5.2-codex"));
    assert!(Gemini::available_models().contains(&"auto"));
    assert!(Copilot::available_models().contains(&"claude-sonnet-4.5"));
}
