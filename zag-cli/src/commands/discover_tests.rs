use super::*;
use zag_agent::capability;

#[test]
fn summary_table_does_not_panic() {
    let caps = get_all_capabilities();
    let out = capability::format_summary_table(&caps);
    assert!(out.contains("PROVIDER"));
    assert!(out.contains("claude"));
}

#[test]
fn provider_detail_does_not_panic() {
    let cap = get_capability("claude").unwrap();
    let out = capability::format_provider_detail(&cap);
    assert!(out.starts_with("Provider: claude"));
    assert!(out.contains("Features:"));
}

#[test]
fn print_models_text_single_provider() {
    print_models(Some("claude"), "text", false).unwrap();
}

#[test]
fn print_models_text_all_providers() {
    print_models(None, "text", false).unwrap();
}

#[test]
fn format_models_json() {
    let caps = get_all_capabilities();
    let output = capability::format_models(&caps, "json", false).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed.len(), 5);
    assert!(parsed[0]["provider"].is_string());
    assert!(parsed[0]["models"].is_array());
}

#[test]
fn format_all_json() {
    let caps = get_all_capabilities();
    let output = capability::format_capabilities(&caps, "json", true).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed.len(), 5);
}

#[test]
fn format_all_yaml() {
    let caps = get_all_capabilities();
    let output = capability::format_capabilities(&caps, "yaml", false).unwrap();
    assert!(output.contains("claude"));
}

#[test]
fn format_all_toml() {
    let caps = get_all_capabilities();
    let output = capability::format_capabilities(&caps, "toml", false).unwrap();
    assert!(output.contains("claude"));
}

#[test]
fn run_discover_default() {
    run_discover(None, false, None, false, None, false).unwrap();
}

#[test]
fn run_discover_json() {
    run_discover(None, false, None, true, None, false).unwrap();
}

#[test]
fn run_discover_single_provider() {
    run_discover(Some("claude"), false, None, false, None, false).unwrap();
}

#[test]
fn run_discover_models() {
    run_discover(None, true, None, false, None, false).unwrap();
}

#[test]
fn run_discover_resolve_alias() {
    run_discover(Some("claude"), false, Some("default"), false, None, false).unwrap();
}

#[test]
fn run_discover_resolve_json() {
    run_discover(Some("claude"), false, Some("small"), true, None, true).unwrap();
}

#[test]
fn run_discover_resolve_requires_provider() {
    let result = run_discover(None, false, Some("default"), false, None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("--provider"));
}
