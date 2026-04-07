use super::*;

#[tokio::test]
async fn check_server_health_returns_false_for_unreachable_server() {
    let config = ConnectConfig {
        url: "https://127.0.0.1:19999".to_string(), // unlikely to be listening
        token: "test-token".to_string(),
        username: None,
    };
    // Clear any cached health check so we actually hit the network
    let _ = std::fs::remove_file(ConnectConfig::health_cache_path());
    let healthy = check_server_health(&config).await;
    assert!(!healthy);
}

#[tokio::test]
async fn check_server_health_uses_cache() {
    // Write a fresh cache timestamp
    let _ = ConnectConfig::update_health_cache();
    let config = ConnectConfig {
        url: "https://127.0.0.1:19999".to_string(), // unreachable, but cache should bypass
        token: "test-token".to_string(),
        username: None,
    };
    let healthy = check_server_health(&config).await;
    assert!(
        healthy,
        "should return true from cache even if server is unreachable"
    );
    // Clean up
    let _ = std::fs::remove_file(ConnectConfig::health_cache_path());
}

#[test]
fn matches_filter_single_match() {
    let event = serde_json::json!({"type": "SessionEnded", "success": "true"});
    assert!(matches_filter(&event, "type=SessionEnded"));
}

#[test]
fn matches_filter_no_match() {
    let event = serde_json::json!({"type": "SessionStarted"});
    assert!(!matches_filter(&event, "type=SessionEnded"));
}

#[test]
fn matches_filter_multiple_conditions() {
    let event = serde_json::json!({"type": "ToolResult", "success": "true"});
    assert!(matches_filter(&event, "type=ToolResult,success=true"));
    assert!(!matches_filter(&event, "type=ToolResult,success=false"));
}
