use super::*;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn make_stdio_server(name: &str, command: &str, args: &[&str]) -> McpServer {
    McpServer {
        name: name.to_string(),
        description: String::new(),
        transport: "stdio".to_string(),
        command: Some(command.to_string()),
        args: args.iter().map(|a| a.to_string()).collect(),
        url: None,
        bearer_token_env_var: None,
        headers: BTreeMap::new(),
        env: BTreeMap::new(),
    }
}

fn make_http_server(name: &str, url: &str) -> McpServer {
    McpServer {
        name: name.to_string(),
        description: String::new(),
        transport: "http".to_string(),
        command: None,
        args: Vec::new(),
        url: Some(url.to_string()),
        bearer_token_env_var: None,
        headers: BTreeMap::new(),
        env: BTreeMap::new(),
    }
}

fn make_server_with_env(name: &str, command: &str, env: &[(&str, &str)]) -> McpServer {
    let mut server = make_stdio_server(name, command, &[]);
    server.env = env
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    server
}

fn write_server_toml(dir: &Path, server: &McpServer) {
    let path = dir.join(format!("{}.toml", server.name));
    let content = toml::to_string_pretty(server).unwrap();
    fs::write(path, content).unwrap();
}

// ---------------------------------------------------------------------------
// Parsing tests
// ---------------------------------------------------------------------------

#[test]
fn test_parse_stdio_server() {
    let dir = TempDir::new().unwrap();
    let toml_content = r#"
name = "github"
description = "GitHub MCP server"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"
"#;
    let path = dir.path().join("github.toml");
    fs::write(&path, toml_content).unwrap();

    let server = parse_server(&path).unwrap();
    assert_eq!(server.name, "github");
    assert_eq!(server.description, "GitHub MCP server");
    assert_eq!(server.transport, "stdio");
    assert_eq!(server.command, Some("npx".to_string()));
    assert_eq!(
        server.args,
        vec!["-y", "@modelcontextprotocol/server-github"]
    );
    assert_eq!(server.env.get("GITHUB_TOKEN").unwrap(), "${GITHUB_TOKEN}");
}

#[test]
fn test_parse_http_server() {
    let dir = TempDir::new().unwrap();
    let toml_content = r#"
name = "sentry"
transport = "http"
url = "https://mcp.sentry.dev/sse"
bearer_token_env_var = "SENTRY_AUTH_TOKEN"
"#;
    let path = dir.path().join("sentry.toml");
    fs::write(&path, toml_content).unwrap();

    let server = parse_server(&path).unwrap();
    assert_eq!(server.name, "sentry");
    assert_eq!(server.transport, "http");
    assert_eq!(server.url, Some("https://mcp.sentry.dev/sse".to_string()));
    assert_eq!(
        server.bearer_token_env_var,
        Some("SENTRY_AUTH_TOKEN".to_string())
    );
}

#[test]
fn test_parse_minimal_server() {
    let dir = TempDir::new().unwrap();
    let toml_content = r#"
name = "test"
command = "test-cmd"
"#;
    let path = dir.path().join("test.toml");
    fs::write(&path, toml_content).unwrap();

    let server = parse_server(&path).unwrap();
    assert_eq!(server.name, "test");
    assert_eq!(server.transport, "stdio"); // default
    assert_eq!(server.command, Some("test-cmd".to_string()));
}

// ---------------------------------------------------------------------------
// Loading tests
// ---------------------------------------------------------------------------

#[test]
fn test_load_servers_from_empty_dir() {
    let dir = TempDir::new().unwrap();
    let servers = load_servers_from_dir(dir.path()).unwrap();
    assert!(servers.is_empty());
}

#[test]
fn test_load_servers_from_nonexistent_dir() {
    let servers = load_servers_from_dir(Path::new("/nonexistent/path")).unwrap();
    assert!(servers.is_empty());
}

#[test]
fn test_load_servers_sorted() {
    let dir = TempDir::new().unwrap();
    write_server_toml(dir.path(), &make_stdio_server("zebra", "cmd", &[]));
    write_server_toml(dir.path(), &make_stdio_server("alpha", "cmd", &[]));
    write_server_toml(dir.path(), &make_stdio_server("middle", "cmd", &[]));

    let servers = load_servers_from_dir(dir.path()).unwrap();
    assert_eq!(servers.len(), 3);
    assert_eq!(servers[0].name, "alpha");
    assert_eq!(servers[1].name, "middle");
    assert_eq!(servers[2].name, "zebra");
}

#[test]
fn test_load_skips_non_toml_files() {
    let dir = TempDir::new().unwrap();
    write_server_toml(dir.path(), &make_stdio_server("valid", "cmd", &[]));
    fs::write(dir.path().join("readme.md"), "not toml").unwrap();
    fs::write(dir.path().join("notes.txt"), "not toml").unwrap();

    let servers = load_servers_from_dir(dir.path()).unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "valid");
}

#[test]
fn test_load_skips_invalid_toml() {
    let dir = TempDir::new().unwrap();
    write_server_toml(dir.path(), &make_stdio_server("valid", "cmd", &[]));
    fs::write(dir.path().join("broken.toml"), "not valid {{toml").unwrap();

    let servers = load_servers_from_dir(dir.path()).unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "valid");
}

// ---------------------------------------------------------------------------
// JSON sync tests (Claude, Gemini, Copilot)
// ---------------------------------------------------------------------------

#[test]
fn test_sync_json_creates_new_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");
    let servers = vec![make_stdio_server("github", "npx", &["-y", "pkg"])];

    let synced = sync_json_provider_to("claude", &servers, &config_path).unwrap();
    assert_eq!(synced, 1);

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    let mcp = config["mcpServers"].as_object().unwrap();
    assert!(mcp.contains_key("zag-github"));
    assert_eq!(mcp["zag-github"]["command"], "npx");
    assert_eq!(mcp["zag-github"]["type"], "stdio");
}

#[test]
fn test_sync_json_preserves_existing_entries() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");

    // Write existing config with a user-managed server
    let existing = serde_json::json!({
        "mcpServers": {
            "my-server": {
                "command": "my-cmd",
                "args": []
            }
        },
        "otherSetting": true
    });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    let servers = vec![make_stdio_server("new-server", "npx", &[])];
    sync_json_provider_to("claude", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();

    // User's server is preserved
    assert!(config["mcpServers"]["my-server"].is_object());
    // Zag server is added
    assert!(config["mcpServers"]["zag-new-server"].is_object());
    // Other settings preserved
    assert_eq!(config["otherSetting"], true);
}

#[test]
fn test_sync_json_removes_stale_zag_entries() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");

    // Write config with old zag entry
    let existing = serde_json::json!({
        "mcpServers": {
            "zag-old-server": { "command": "old" },
            "my-server": { "command": "mine" }
        }
    });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    // Sync with different servers
    let servers = vec![make_stdio_server("new-server", "npx", &[])];
    sync_json_provider_to("claude", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    let mcp = config["mcpServers"].as_object().unwrap();

    // Old zag entry removed
    assert!(!mcp.contains_key("zag-old-server"));
    // New zag entry added
    assert!(mcp.contains_key("zag-new-server"));
    // User entry preserved
    assert!(mcp.contains_key("my-server"));
}

#[test]
fn test_sync_json_http_server() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");
    let servers = vec![make_http_server("sentry", "https://mcp.sentry.dev/sse")];

    sync_json_provider_to("claude", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    let entry = &config["mcpServers"]["zag-sentry"];
    assert_eq!(entry["type"], "http");
    assert_eq!(entry["url"], "https://mcp.sentry.dev/sse");
}

#[test]
fn test_sync_json_copilot_uses_local_type() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");
    let servers = vec![make_stdio_server("test", "cmd", &[])];

    sync_json_provider_to("copilot", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(config["mcpServers"]["zag-test"]["type"], "local");
}

#[test]
fn test_sync_json_gemini_omits_type_for_stdio() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");
    let servers = vec![make_stdio_server("test", "cmd", &[])];

    sync_json_provider_to("gemini", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    // Gemini doesn't use "type" field for stdio
    assert!(config["mcpServers"]["zag-test"].get("type").is_none());
    assert_eq!(config["mcpServers"]["zag-test"]["command"], "cmd");
}

#[test]
fn test_sync_json_gemini_uses_http_url() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");
    let servers = vec![make_http_server("remote", "https://example.com/mcp")];

    sync_json_provider_to("gemini", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    // Gemini uses httpUrl instead of url
    assert_eq!(
        config["mcpServers"]["zag-remote"]["httpUrl"],
        "https://example.com/mcp"
    );
}

#[test]
fn test_sync_json_with_env_vars() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");
    let servers = vec![make_server_with_env("gh", "npx", &[("TOKEN", "abc")])];

    sync_json_provider_to("claude", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(config["mcpServers"]["zag-gh"]["env"]["TOKEN"], "abc");
}

// ---------------------------------------------------------------------------
// Codex TOML sync tests
// ---------------------------------------------------------------------------

#[test]
fn test_sync_codex_creates_new_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let servers = vec![make_stdio_server("github", "npx", &["-y", "pkg"])];

    let synced = sync_codex_provider_to(&servers, &config_path).unwrap();
    assert_eq!(synced, 1);

    let content = fs::read_to_string(&config_path).unwrap();
    let config: toml::Table = content.parse().unwrap();
    let mcp = config["mcp_servers"].as_table().unwrap();
    assert!(mcp.contains_key("zag-github"));
    let entry = mcp["zag-github"].as_table().unwrap();
    assert_eq!(entry["command"].as_str().unwrap(), "npx");
}

#[test]
fn test_sync_codex_preserves_existing_entries() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let existing = r#"
[other_setting]
key = "value"

[mcp_servers.my-server]
command = "my-cmd"
"#;
    fs::write(&config_path, existing).unwrap();

    let servers = vec![make_stdio_server("new-server", "npx", &[])];
    sync_codex_provider_to(&servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: toml::Table = content.parse().unwrap();

    // User's server preserved
    let mcp = config["mcp_servers"].as_table().unwrap();
    assert!(mcp.contains_key("my-server"));
    // Zag server added
    assert!(mcp.contains_key("zag-new-server"));
    // Other settings preserved
    assert!(config.contains_key("other_setting"));
}

#[test]
fn test_sync_codex_removes_stale_zag_entries() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let existing = r#"
[mcp_servers.zag-old]
command = "old"

[mcp_servers.user-server]
command = "mine"
"#;
    fs::write(&config_path, existing).unwrap();

    let servers = vec![make_stdio_server("new", "npx", &[])];
    sync_codex_provider_to(&servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: toml::Table = content.parse().unwrap();
    let mcp = config["mcp_servers"].as_table().unwrap();

    assert!(!mcp.contains_key("zag-old"));
    assert!(mcp.contains_key("zag-new"));
    assert!(mcp.contains_key("user-server"));
}

#[test]
fn test_sync_codex_http_server() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let mut server = make_http_server("sentry", "https://mcp.sentry.dev/sse");
    server.bearer_token_env_var = Some("SENTRY_TOKEN".to_string());

    sync_codex_provider_to(&[server], &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: toml::Table = content.parse().unwrap();
    let entry = config["mcp_servers"]["zag-sentry"].as_table().unwrap();
    assert_eq!(entry["url"].as_str().unwrap(), "https://mcp.sentry.dev/sse");
    assert_eq!(
        entry["bearer_token_env_var"].as_str().unwrap(),
        "SENTRY_TOKEN"
    );
}

#[test]
fn test_sync_codex_with_env_vars() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    let servers = vec![make_server_with_env("gh", "npx", &[("TOKEN", "abc")])];

    sync_codex_provider_to(&servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: toml::Table = content.parse().unwrap();
    let entry = config["mcp_servers"]["zag-gh"].as_table().unwrap();
    let env = entry["env"].as_table().unwrap();
    assert_eq!(env["TOKEN"].as_str().unwrap(), "abc");
}

// ---------------------------------------------------------------------------
// Import from JSON tests
// ---------------------------------------------------------------------------

#[test]
fn test_import_from_json() {
    let src = TempDir::new().unwrap();
    let dest = TempDir::new().unwrap();

    let config = serde_json::json!({
        "mcpServers": {
            "github": { "command": "npx", "args": ["-y", "pkg"], "env": {"TOKEN": "x"} },
            "sentry": { "url": "https://sentry.dev/mcp" }
        }
    });
    let config_path = src.path().join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    let imported = import_from_json_to(&config_path, "claude", dest.path()).unwrap();
    assert_eq!(imported.len(), 2);
    assert!(imported.contains(&"github".to_string()));
    assert!(imported.contains(&"sentry".to_string()));

    // Verify the created TOML files
    let gh = parse_server(&dest.path().join("github.toml")).unwrap();
    assert_eq!(gh.name, "github");
    assert_eq!(gh.transport, "stdio");
    assert_eq!(gh.command, Some("npx".to_string()));
    assert_eq!(gh.env.get("TOKEN").unwrap(), "x");

    let sentry = parse_server(&dest.path().join("sentry.toml")).unwrap();
    assert_eq!(sentry.name, "sentry");
    assert_eq!(sentry.transport, "http");
    assert_eq!(sentry.url, Some("https://sentry.dev/mcp".to_string()));
}

#[test]
fn test_import_from_json_skips_zag_prefixed() {
    let src = TempDir::new().unwrap();
    let dest = TempDir::new().unwrap();

    let config = serde_json::json!({
        "mcpServers": {
            "zag-managed": { "command": "npx" },
            "user-server": { "command": "cmd" }
        }
    });
    let config_path = src.path().join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    let imported = import_from_json_to(&config_path, "claude", dest.path()).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0], "user-server");
}

#[test]
fn test_import_from_json_skips_existing() {
    let src = TempDir::new().unwrap();
    let dest = TempDir::new().unwrap();

    // Pre-create an existing server
    let existing = make_stdio_server("github", "existing", &[]);
    write_server_toml(dest.path(), &existing);

    let config = serde_json::json!({
        "mcpServers": {
            "github": { "command": "new-cmd" },
            "sentry": { "command": "sentry-cmd" }
        }
    });
    let config_path = src.path().join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    let imported = import_from_json_to(&config_path, "claude", dest.path()).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0], "sentry");

    // Original github.toml should be unchanged
    let gh = parse_server(&dest.path().join("github.toml")).unwrap();
    assert_eq!(gh.command, Some("existing".to_string()));
}

#[test]
fn test_import_from_json_no_mcp_servers() {
    let src = TempDir::new().unwrap();
    let dest = TempDir::new().unwrap();

    let config = serde_json::json!({ "otherSetting": true });
    let config_path = src.path().join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    let imported = import_from_json_to(&config_path, "claude", dest.path()).unwrap();
    assert!(imported.is_empty());
}

// ---------------------------------------------------------------------------
// Import from Codex TOML tests
// ---------------------------------------------------------------------------

#[test]
fn test_import_from_codex_toml() {
    let src = TempDir::new().unwrap();
    let dest = TempDir::new().unwrap();

    let config = r#"
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]

[mcp_servers.sentry]
url = "https://mcp.sentry.dev/sse"
bearer_token_env_var = "SENTRY_TOKEN"
"#;
    let config_path = src.path().join("config.toml");
    fs::write(&config_path, config).unwrap();

    let imported = import_from_codex_to(&config_path, dest.path()).unwrap();
    assert_eq!(imported.len(), 2);

    let fs_server = parse_server(&dest.path().join("filesystem.toml")).unwrap();
    assert_eq!(fs_server.transport, "stdio");
    assert_eq!(fs_server.command, Some("npx".to_string()));

    let sentry = parse_server(&dest.path().join("sentry.toml")).unwrap();
    assert_eq!(sentry.transport, "http");
    assert_eq!(
        sentry.bearer_token_env_var,
        Some("SENTRY_TOKEN".to_string())
    );
}

#[test]
fn test_import_from_codex_skips_zag_prefixed() {
    let src = TempDir::new().unwrap();
    let dest = TempDir::new().unwrap();

    let config = r#"
[mcp_servers.zag-managed]
command = "npx"

[mcp_servers.user-server]
command = "cmd"
"#;
    let config_path = src.path().join("config.toml");
    fs::write(&config_path, config).unwrap();

    let imported = import_from_codex_to(&config_path, dest.path()).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0], "user-server");
}

// ---------------------------------------------------------------------------
// Sync empty list removes all zag- entries
// ---------------------------------------------------------------------------

#[test]
fn test_sync_empty_removes_all_zag_entries() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.json");

    let existing = serde_json::json!({
        "mcpServers": {
            "zag-old": { "command": "old" },
            "user-server": { "command": "mine" }
        }
    });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    let servers: Vec<McpServer> = vec![];
    sync_json_provider_to("claude", &servers, &config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap();
    let mcp = config["mcpServers"].as_object().unwrap();

    assert!(!mcp.contains_key("zag-old"));
    assert!(mcp.contains_key("user-server"));
}

// ---------------------------------------------------------------------------
// Round-trip test: add → sync → import
// ---------------------------------------------------------------------------

#[test]
fn test_roundtrip_add_sync_import() {
    let mcp_store = TempDir::new().unwrap();
    let provider_config_dir = TempDir::new().unwrap();
    let import_dest = TempDir::new().unwrap();

    // Create a server
    let server = make_server_with_env("context7", "npx", &[("API_KEY", "secret")]);
    write_server_toml(mcp_store.path(), &server);

    // Load and sync to Claude JSON
    let servers = load_servers_from_dir(mcp_store.path()).unwrap();
    let config_path = provider_config_dir.path().join("claude.json");
    sync_json_provider_to("claude", &servers, &config_path).unwrap();

    // Now import from that Claude config
    let imported = import_from_json_to(&config_path, "claude", import_dest.path()).unwrap();
    // The synced entry has zag- prefix, so it should be skipped during import
    assert!(imported.is_empty());

    // Add a user entry to the Claude config
    let content = fs::read_to_string(&config_path).unwrap();
    let mut config: serde_json::Value = serde_json::from_str(&content).unwrap();
    config["mcpServers"]["user-manual"] = serde_json::json!({"command": "manual-cmd"});
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Now import should pick up the user entry
    let imported = import_from_json_to(&config_path, "claude", import_dest.path()).unwrap();
    assert_eq!(imported, vec!["user-manual"]);
}
