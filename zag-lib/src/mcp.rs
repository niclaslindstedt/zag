/// Provider-agnostic MCP (Model Context Protocol) server management.
///
/// MCP server configs are stored as individual TOML files:
/// - Global: `~/.zag/mcp/<server-name>.toml`
/// - Project-scoped: `~/.zag/projects/<sanitized-path>/mcp/<server-name>.toml`
///
/// During sync, servers are injected into each provider's native config format
/// with a `zag-` prefix to avoid collisions with user-managed servers.
///
/// Supported providers:
/// - Claude: `~/.claude.json` under `mcpServers` (JSON)
/// - Gemini: `~/.gemini/settings.json` under `mcpServers` (JSON)
/// - Copilot: `~/.copilot/mcp-config.json` under `mcpServers` (JSON)
/// - Codex: `~/.codex/config.toml` under `[mcp_servers]` (TOML)
/// - Ollama: No native MCP support
#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const MCP_PREFIX: &str = "zag-";

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// An MCP server configuration (one per TOML file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Human-readable name (also the filename stem).
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: String,
    /// Transport type: "stdio" or "http".
    #[serde(default = "default_transport")]
    pub transport: String,

    // -- stdio fields --
    /// Command to start the server (stdio transport).
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,

    // -- http fields --
    /// URL endpoint (http transport).
    #[serde(default)]
    pub url: Option<String>,
    /// Environment variable name containing a bearer token (http transport).
    #[serde(default)]
    pub bearer_token_env_var: Option<String>,
    /// HTTP headers for http transport.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,

    // -- shared --
    /// Environment variables forwarded to the server process.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

fn default_transport() -> String {
    "stdio".to_string()
}

// ---------------------------------------------------------------------------
// Directory helpers
// ---------------------------------------------------------------------------

/// Returns `~/.zag/mcp/`.
pub fn mcp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".zag")
        .join("mcp")
}

/// Returns the project-scoped MCP directory for the given root.
/// Uses the same sanitization as config: `~/.zag/projects/<sanitized-path>/mcp/`.
pub fn project_mcp_dir(root: Option<&str>) -> Option<PathBuf> {
    let base = dirs::home_dir()?.join(".zag");

    let project_dir = if let Some(r) = root {
        let sanitized = crate::config::Config::sanitize_path(r);
        base.join("projects").join(sanitized)
    } else {
        let current_dir = std::env::current_dir().ok()?;
        let git_root = find_git_root(&current_dir)?;
        let sanitized = crate::config::Config::sanitize_path(&git_root.to_string_lossy());
        base.join("projects").join(sanitized)
    };

    Some(project_dir.join("mcp"))
}

fn find_git_root(start_dir: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(start_dir)
        .output()
        .ok()?;
    if output.status.success() {
        let root = String::from_utf8(output.stdout).ok()?;
        Some(PathBuf::from(root.trim()))
    } else {
        None
    }
}

/// Returns the provider's native MCP config file path, or `None` if unsupported.
///
/// - Claude: `~/.claude.json`
/// - Gemini: `~/.gemini/settings.json`
/// - Copilot: `~/.copilot/mcp-config.json`
/// - Codex: `~/.codex/config.toml`
pub fn provider_mcp_config_path(provider: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match provider {
        "claude" => Some(home.join(".claude.json")),
        "gemini" => Some(home.join(".gemini").join("settings.json")),
        "copilot" => Some(home.join(".copilot").join("mcp-config.json")),
        "codex" => Some(home.join(".codex").join("config.toml")),
        _ => None,
    }
}

/// List of providers that support MCP.
pub const MCP_PROVIDERS: &[&str] = &["claude", "gemini", "copilot", "codex"];

// ---------------------------------------------------------------------------
// Loading / saving individual servers
// ---------------------------------------------------------------------------

/// Parse an MCP server from a TOML file.
pub fn parse_server(path: &Path) -> Result<McpServer> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let server: McpServer = toml::from_str(&content)
        .with_context(|| format!("Failed to parse MCP server config {}", path.display()))?;
    Ok(server)
}

/// Load all MCP servers from a directory. Silently skips invalid files.
fn load_servers_from(dir: &Path) -> Result<Vec<McpServer>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut servers = Vec::new();
    for entry in fs::read_dir(dir)
        .with_context(|| format!("Failed to read MCP directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        match parse_server(&path) {
            Ok(server) => servers.push(server),
            Err(e) => {
                log::warn!("Skipping MCP server at {}: {}", path.display(), e);
            }
        }
    }
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(servers)
}

/// Load all global MCP servers from `~/.zag/mcp/`.
pub fn load_global_servers() -> Result<Vec<McpServer>> {
    load_servers_from(&mcp_dir())
}

/// Load project-scoped MCP servers. Returns empty vec if not in a project.
pub fn load_project_servers(root: Option<&str>) -> Result<Vec<McpServer>> {
    match project_mcp_dir(root) {
        Some(dir) => load_servers_from(&dir),
        None => Ok(Vec::new()),
    }
}

/// Load all MCP servers (global + project-scoped, project overrides global).
pub fn load_all_servers(root: Option<&str>) -> Result<Vec<McpServer>> {
    let mut by_name: BTreeMap<String, McpServer> = BTreeMap::new();

    for server in load_global_servers()? {
        by_name.insert(server.name.clone(), server);
    }
    for server in load_project_servers(root)? {
        by_name.insert(server.name.clone(), server);
    }

    Ok(by_name.into_values().collect())
}

/// List all MCP servers (alias for load_all_servers).
pub fn list_servers(root: Option<&str>) -> Result<Vec<McpServer>> {
    load_all_servers(root)
}

/// Get a single MCP server by name. Checks project-scoped first, then global.
pub fn get_server(name: &str, root: Option<&str>) -> Result<McpServer> {
    // Check project-scoped first
    if let Some(dir) = project_mcp_dir(root) {
        let path = dir.join(format!("{}.toml", name));
        if path.exists() {
            return parse_server(&path);
        }
    }
    // Check global
    let path = mcp_dir().join(format!("{}.toml", name));
    if path.exists() {
        return parse_server(&path);
    }
    bail!("MCP server '{}' not found", name);
}

/// Create a new MCP server config file. Returns the path to the new file.
/// If `project` is true, creates in project-scoped dir; otherwise global.
pub fn add_server(server: &McpServer, project: bool, root: Option<&str>) -> Result<PathBuf> {
    let dir = if project {
        project_mcp_dir(root).context("Not in a project (no git root found)")?
    } else {
        mcp_dir()
    };
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create MCP directory {}", dir.display()))?;

    let path = dir.join(format!("{}.toml", server.name));
    if path.exists() {
        bail!(
            "MCP server '{}' already exists at {}",
            server.name,
            path.display()
        );
    }

    let content =
        toml::to_string_pretty(server).context("Failed to serialize MCP server config")?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(path)
}

/// Remove an MCP server config file and clean up provider configs.
pub fn remove_server(name: &str, root: Option<&str>) -> Result<()> {
    let mut found = false;

    // Try project-scoped first
    if let Some(dir) = project_mcp_dir(root) {
        let path = dir.join(format!("{}.toml", name));
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove {}", path.display()))?;
            found = true;
        }
    }

    // Try global
    let path = mcp_dir().join(format!("{}.toml", name));
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;
        found = true;
    }

    if !found {
        bail!("MCP server '{}' not found", name);
    }

    // Remove from all provider configs
    for provider in MCP_PROVIDERS {
        if let Err(e) = remove_server_from_provider(provider, name) {
            log::warn!(
                "Failed to clean up {} config for '{}': {}",
                provider,
                name,
                e
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Provider sync: convert McpServer → provider-native format
// ---------------------------------------------------------------------------

/// Convert an MCP server to a Claude/Gemini/Copilot JSON entry (serde_json::Value).
fn server_to_json(server: &McpServer, provider: &str) -> serde_json::Value {
    let mut entry = serde_json::Map::new();

    if server.transport == "stdio" {
        if let Some(ref cmd) = server.command {
            // Copilot uses "type": "local", Claude uses "type": "stdio", Gemini omits type
            match provider {
                "copilot" => {
                    entry.insert("type".into(), serde_json::json!("local"));
                }
                "claude" => {
                    entry.insert("type".into(), serde_json::json!("stdio"));
                }
                _ => {}
            }
            entry.insert("command".into(), serde_json::json!(cmd));
            if !server.args.is_empty() {
                entry.insert("args".into(), serde_json::json!(server.args));
            }
        }
    } else if server.transport == "http" {
        if let Some(ref url) = server.url {
            match provider {
                "copilot" => {
                    entry.insert("type".into(), serde_json::json!("http"));
                    entry.insert("url".into(), serde_json::json!(url));
                }
                "gemini" => {
                    entry.insert("httpUrl".into(), serde_json::json!(url));
                }
                _ => {
                    entry.insert("type".into(), serde_json::json!("http"));
                    entry.insert("url".into(), serde_json::json!(url));
                }
            }
        }
        if !server.headers.is_empty() {
            entry.insert("headers".into(), serde_json::json!(server.headers));
        }
    }

    if !server.env.is_empty() {
        entry.insert("env".into(), serde_json::json!(server.env));
    }

    serde_json::Value::Object(entry)
}

/// Sync all MCP servers into a JSON-based provider's config file.
/// Only touches entries with the `zag-` prefix.
fn sync_json_provider(provider: &str, servers: &[McpServer]) -> Result<usize> {
    let Some(config_path) = provider_mcp_config_path(provider) else {
        return Ok(0);
    };

    // Read existing config (or start with empty object)
    let mut config: serde_json::Value = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    let mcp_servers = config
        .as_object_mut()
        .context("Config is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let mcp_map = mcp_servers
        .as_object_mut()
        .context("mcpServers is not a JSON object")?;

    // Remove all existing zag- entries
    let zag_keys: Vec<String> = mcp_map
        .keys()
        .filter(|k| k.starts_with(MCP_PREFIX))
        .cloned()
        .collect();
    for key in &zag_keys {
        mcp_map.remove(key);
    }

    // Add current servers with zag- prefix
    let mut synced = 0;
    for server in servers {
        let key = format!("{}{}", MCP_PREFIX, server.name);
        let value = server_to_json(server, provider);
        mcp_map.insert(key, value);
        synced += 1;
    }

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write back with pretty printing
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, format!("{}\n", content))
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    log::debug!(
        "Synced {} MCP server(s) to {} at {}",
        synced,
        provider,
        config_path.display()
    );

    Ok(synced)
}

/// Sync all MCP servers into Codex's TOML config.
/// Only touches entries under `[mcp_servers]` with the `zag-` prefix.
fn sync_codex_provider(servers: &[McpServer]) -> Result<usize> {
    let Some(config_path) = provider_mcp_config_path("codex") else {
        return Ok(0);
    };

    // Read existing config as a TOML table
    let mut config: toml::Table = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        content
            .parse::<toml::Table>()
            .with_context(|| format!("Failed to parse {}", config_path.display()))?
    } else {
        toml::Table::new()
    };

    // Get or create mcp_servers table
    let mcp_table = config
        .entry("mcp_servers")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .context("mcp_servers is not a TOML table")?;

    // Remove all existing zag- entries
    let zag_keys: Vec<String> = mcp_table
        .keys()
        .filter(|k| k.starts_with(MCP_PREFIX))
        .cloned()
        .collect();
    for key in &zag_keys {
        mcp_table.remove(key.as_str());
    }

    // Add current servers with zag- prefix
    let mut synced = 0;
    for server in servers {
        let key = format!("{}{}", MCP_PREFIX, server.name);
        let mut entry = toml::Table::new();

        if server.transport == "stdio" {
            if let Some(ref cmd) = server.command {
                entry.insert("command".into(), toml::Value::String(cmd.clone()));
            }
            if !server.args.is_empty() {
                let args: Vec<toml::Value> = server
                    .args
                    .iter()
                    .map(|a| toml::Value::String(a.clone()))
                    .collect();
                entry.insert("args".into(), toml::Value::Array(args));
            }
        } else if server.transport == "http" {
            if let Some(ref url) = server.url {
                entry.insert("url".into(), toml::Value::String(url.clone()));
            }
            if let Some(ref token_var) = server.bearer_token_env_var {
                entry.insert(
                    "bearer_token_env_var".into(),
                    toml::Value::String(token_var.clone()),
                );
            }
        }

        if !server.env.is_empty() {
            let mut env_table = toml::Table::new();
            for (k, v) in &server.env {
                env_table.insert(k.clone(), toml::Value::String(v.clone()));
            }
            entry.insert("env".into(), toml::Value::Table(env_table));
        }

        mcp_table.insert(key, toml::Value::Table(entry));
        synced += 1;
    }

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(&config)?;
    fs::write(&config_path, &content)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    log::debug!(
        "Synced {} MCP server(s) to codex at {}",
        synced,
        config_path.display()
    );

    Ok(synced)
}

/// Sync MCP servers for a specific provider.
pub fn sync_servers_for_provider(provider: &str, servers: &[McpServer]) -> Result<usize> {
    match provider {
        "claude" | "gemini" | "copilot" => sync_json_provider(provider, servers),
        "codex" => sync_codex_provider(servers),
        _ => {
            log::debug!("Provider '{}' does not support MCP servers", provider);
            Ok(0)
        }
    }
}

/// Remove a single zag-managed server from a provider's config.
fn remove_server_from_provider(provider: &str, name: &str) -> Result<()> {
    let Some(config_path) = provider_mcp_config_path(provider) else {
        return Ok(());
    };
    if !config_path.exists() {
        return Ok(());
    }

    let key = format!("{}{}", MCP_PREFIX, name);

    if provider == "codex" {
        let content = fs::read_to_string(&config_path)?;
        let mut config: toml::Table = content.parse()?;
        if let Some(mcp) = config.get_mut("mcp_servers").and_then(|v| v.as_table_mut()) {
            mcp.remove(&key);
        }
        let content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, &content)?;
    } else {
        let content = fs::read_to_string(&config_path)?;
        let mut config: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(mcp) = config
            .as_object_mut()
            .and_then(|o| o.get_mut("mcpServers"))
            .and_then(|v| v.as_object_mut())
        {
            mcp.remove(&key);
        }
        let content = serde_json::to_string_pretty(&config)?;
        fs::write(&config_path, format!("{}\n", content))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Import from provider configs
// ---------------------------------------------------------------------------

/// Import MCP servers from a provider's native config into `~/.zag/mcp/`.
/// Skips entries prefixed with `zag-` (our own entries).
/// Returns names of imported servers.
pub fn import_servers(from_provider: &str) -> Result<Vec<String>> {
    let Some(config_path) = provider_mcp_config_path(from_provider) else {
        bail!("Provider '{}' does not support MCP servers", from_provider);
    };

    if !config_path.exists() {
        bail!(
            "No MCP config found for '{}' at {}",
            from_provider,
            config_path.display()
        );
    }

    if from_provider == "codex" {
        import_from_codex_toml(&config_path)
    } else {
        import_from_json(&config_path, from_provider)
    }
}

/// Import MCP servers from a JSON config (Claude, Gemini, Copilot).
fn import_from_json(config_path: &Path, provider: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;

    let mcp_servers = match config.get("mcpServers").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return Ok(Vec::new()),
    };

    let dest_dir = mcp_dir();
    fs::create_dir_all(&dest_dir)?;

    let mut imported = Vec::new();

    for (name, value) in mcp_servers {
        // Skip our own entries
        if name.starts_with(MCP_PREFIX) {
            continue;
        }

        let dest = dest_dir.join(format!("{}.toml", name));
        if dest.exists() {
            log::debug!("Skipping '{}': already exists in ~/.zag/mcp/", name);
            continue;
        }

        let server = json_entry_to_server(name, value, provider);
        let content = toml::to_string_pretty(&server).context("Failed to serialize MCP server")?;
        fs::write(&dest, content).with_context(|| format!("Failed to write {}", dest.display()))?;

        imported.push(name.clone());
    }

    Ok(imported)
}

/// Convert a JSON mcpServers entry to our McpServer struct.
fn json_entry_to_server(name: &str, value: &serde_json::Value, provider: &str) -> McpServer {
    let obj = value.as_object();

    // Detect transport
    let transport = if obj.and_then(|o| o.get("url")).is_some()
        || obj.and_then(|o| o.get("httpUrl")).is_some()
    {
        "http".to_string()
    } else {
        "stdio".to_string()
    };

    let command = obj
        .and_then(|o| o.get("command"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let args = obj
        .and_then(|o| o.get("args"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let url = obj
        .and_then(|o| o.get("url").or_else(|| o.get("httpUrl")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let env = obj
        .and_then(|o| o.get("env"))
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let headers = obj
        .and_then(|o| o.get("headers"))
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let _ = provider; // reserved for provider-specific quirks

    McpServer {
        name: name.to_string(),
        description: String::new(),
        transport,
        command,
        args,
        url,
        bearer_token_env_var: None,
        headers,
        env,
    }
}

/// Import MCP servers from Codex TOML config.
fn import_from_codex_toml(config_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(config_path)?;
    let config: toml::Table = content
        .parse()
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;

    let mcp_servers = match config.get("mcp_servers").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };

    let dest_dir = mcp_dir();
    fs::create_dir_all(&dest_dir)?;

    let mut imported = Vec::new();

    for (name, value) in mcp_servers {
        if name.starts_with(MCP_PREFIX) {
            continue;
        }

        let dest = dest_dir.join(format!("{}.toml", name));
        if dest.exists() {
            log::debug!("Skipping '{}': already exists in ~/.zag/mcp/", name);
            continue;
        }

        let server = toml_entry_to_server(name, value);
        let content = toml::to_string_pretty(&server)?;
        fs::write(&dest, content).with_context(|| format!("Failed to write {}", dest.display()))?;

        imported.push(name.clone());
    }

    Ok(imported)
}

/// Convert a Codex TOML mcp_servers entry to our McpServer struct.
fn toml_entry_to_server(name: &str, value: &toml::Value) -> McpServer {
    let table = value.as_table();

    let transport = if table.and_then(|t| t.get("url")).is_some() {
        "http".to_string()
    } else {
        "stdio".to_string()
    };

    let command = table
        .and_then(|t| t.get("command"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let args = table
        .and_then(|t| t.get("args"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let url = table
        .and_then(|t| t.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let bearer_token_env_var = table
        .and_then(|t| t.get("bearer_token_env_var"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let env = table
        .and_then(|t| t.get("env"))
        .and_then(|v| v.as_table())
        .map(|t| {
            t.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    McpServer {
        name: name.to_string(),
        description: String::new(),
        transport,
        command,
        args,
        url,
        bearer_token_env_var,
        headers: BTreeMap::new(),
        env,
    }
}

// ---------------------------------------------------------------------------
// Orchestration (called automatically before agent sessions)
// ---------------------------------------------------------------------------

/// Set up MCP servers for the given provider. Called before each agent session.
pub fn setup_mcp(provider: &str, root: Option<&str>) -> Result<()> {
    let servers = load_all_servers(root)?;
    if servers.is_empty() {
        return Ok(());
    }

    let synced = sync_servers_for_provider(provider, &servers)?;
    if synced > 0 {
        log::info!("Synced {} MCP server(s) for {}", synced, provider);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Testable variants (accept custom directories)
// ---------------------------------------------------------------------------

/// Load servers from a custom base directory (for testing).
pub fn load_servers_from_dir(dir: &Path) -> Result<Vec<McpServer>> {
    load_servers_from(dir)
}

/// Sync MCP servers to a JSON-format provider config at a custom path (for testing).
pub fn sync_json_provider_to(
    provider: &str,
    servers: &[McpServer],
    config_path: &Path,
) -> Result<usize> {
    let mut config: serde_json::Value = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    let mcp_servers = config
        .as_object_mut()
        .context("Config is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let mcp_map = mcp_servers
        .as_object_mut()
        .context("mcpServers is not a JSON object")?;

    // Remove existing zag- entries
    let zag_keys: Vec<String> = mcp_map
        .keys()
        .filter(|k| k.starts_with(MCP_PREFIX))
        .cloned()
        .collect();
    for key in &zag_keys {
        mcp_map.remove(key);
    }

    let mut synced = 0;
    for server in servers {
        let key = format!("{}{}", MCP_PREFIX, server.name);
        let value = server_to_json(server, provider);
        mcp_map.insert(key, value);
        synced += 1;
    }

    let content = serde_json::to_string_pretty(&config)?;
    fs::write(config_path, format!("{}\n", content))?;
    Ok(synced)
}

/// Sync MCP servers to a Codex TOML config at a custom path (for testing).
pub fn sync_codex_provider_to(servers: &[McpServer], config_path: &Path) -> Result<usize> {
    let mut config: toml::Table = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        content.parse()?
    } else {
        toml::Table::new()
    };

    let mcp_table = config
        .entry("mcp_servers")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .context("mcp_servers is not a TOML table")?;

    let zag_keys: Vec<String> = mcp_table
        .keys()
        .filter(|k| k.starts_with(MCP_PREFIX))
        .cloned()
        .collect();
    for key in &zag_keys {
        mcp_table.remove(key.as_str());
    }

    let mut synced = 0;
    for server in servers {
        let key = format!("{}{}", MCP_PREFIX, server.name);
        let mut entry = toml::Table::new();

        if server.transport == "stdio" {
            if let Some(ref cmd) = server.command {
                entry.insert("command".into(), toml::Value::String(cmd.clone()));
            }
            if !server.args.is_empty() {
                let args: Vec<toml::Value> = server
                    .args
                    .iter()
                    .map(|a| toml::Value::String(a.clone()))
                    .collect();
                entry.insert("args".into(), toml::Value::Array(args));
            }
        } else if server.transport == "http" {
            if let Some(ref url) = server.url {
                entry.insert("url".into(), toml::Value::String(url.clone()));
            }
            if let Some(ref token_var) = server.bearer_token_env_var {
                entry.insert(
                    "bearer_token_env_var".into(),
                    toml::Value::String(token_var.clone()),
                );
            }
        }

        if !server.env.is_empty() {
            let mut env_table = toml::Table::new();
            for (k, v) in &server.env {
                env_table.insert(k.clone(), toml::Value::String(v.clone()));
            }
            entry.insert("env".into(), toml::Value::Table(env_table));
        }

        mcp_table.insert(key, toml::Value::Table(entry));
        synced += 1;
    }

    let content = toml::to_string_pretty(&config)?;
    fs::write(config_path, &content)?;
    Ok(synced)
}

/// Import MCP servers from a JSON config file to a custom destination (for testing).
pub fn import_from_json_to(
    config_path: &Path,
    provider: &str,
    dest_dir: &Path,
) -> Result<Vec<String>> {
    let content = fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;

    let mcp_servers = match config.get("mcpServers").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return Ok(Vec::new()),
    };

    fs::create_dir_all(dest_dir)?;
    let mut imported = Vec::new();

    for (name, value) in mcp_servers {
        if name.starts_with(MCP_PREFIX) {
            continue;
        }
        let dest = dest_dir.join(format!("{}.toml", name));
        if dest.exists() {
            continue;
        }
        let server = json_entry_to_server(name, value, provider);
        let content = toml::to_string_pretty(&server)?;
        fs::write(&dest, content)?;
        imported.push(name.clone());
    }

    Ok(imported)
}

/// Import MCP servers from a Codex TOML config file to a custom destination (for testing).
pub fn import_from_codex_to(config_path: &Path, dest_dir: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(config_path)?;
    let config: toml::Table = content.parse()?;

    let mcp_servers = match config.get("mcp_servers").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return Ok(Vec::new()),
    };

    fs::create_dir_all(dest_dir)?;
    let mut imported = Vec::new();

    for (name, value) in mcp_servers {
        if name.starts_with(MCP_PREFIX) {
            continue;
        }
        let dest = dest_dir.join(format!("{}.toml", name));
        if dest.exists() {
            continue;
        }
        let server = toml_entry_to_server(name, value);
        let content = toml::to_string_pretty(&server)?;
        fs::write(&dest, content)?;
        imported.push(name.clone());
    }

    Ok(imported)
}
