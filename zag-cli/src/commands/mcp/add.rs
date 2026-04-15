use anyhow::{Result, bail};
use zag_agent::mcp;

#[allow(clippy::too_many_arguments)]
pub(crate) fn run(
    name: &str,
    transport: &str,
    command: Option<String>,
    args: Vec<String>,
    url: Option<String>,
    env: Vec<String>,
    description: Option<String>,
    global: bool,
    root: Option<&str>,
) -> Result<()> {
    let project = !global;
    let mut env_map = std::collections::BTreeMap::new();
    for pair in &env {
        if let Some((k, v)) = pair.split_once('=') {
            env_map.insert(k.to_string(), v.to_string());
        } else {
            bail!("Invalid --env format '{pair}'. Expected KEY=VALUE");
        }
    }
    let server = mcp::McpServer {
        name: name.to_string(),
        description: description.unwrap_or_default(),
        transport: transport.to_string(),
        command,
        args,
        url,
        bearer_token_env_var: None,
        headers: std::collections::BTreeMap::new(),
        env: env_map,
    };
    let path = mcp::add_server(&server, project, root)?;
    println!(
        "\x1b[32m✓\x1b[0m Created MCP server '{}' at {}",
        name,
        path.display()
    );
    println!(
        "Edit {} to add environment variables or customize.",
        path.display()
    );
    Ok(())
}
