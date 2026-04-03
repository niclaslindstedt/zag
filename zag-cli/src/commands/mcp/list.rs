use anyhow::Result;
use zag::mcp;

pub(crate) fn run(json: bool, root: Option<&str>) -> Result<()> {
    let servers = mcp::list_servers(root)?;
    if json {
        println!("{}", serde_json::to_string(&servers)?);
        return Ok(());
    }
    if servers.is_empty() {
        println!("No MCP servers found in {}", mcp::mcp_dir().display());
        println!(
            "Use 'zag mcp add <name>' to create one, or 'zag mcp import' to import from a provider."
        );
        return Ok(());
    }
    println!(
        "{:<20} {:<10} {:<30} DESCRIPTION",
        "NAME", "TRANSPORT", "COMMAND/URL"
    );
    println!("{}", "-".repeat(90));
    for server in &servers {
        let target = if server.transport == "stdio" {
            server.command.as_deref().unwrap_or("-")
        } else {
            server.url.as_deref().unwrap_or("-")
        };
        let target_display = if target.len() > 28 {
            format!("{}...", &target[..28])
        } else {
            target.to_string()
        };
        let desc = if server.description.len() > 30 {
            format!("{}...", &server.description[..30])
        } else {
            server.description.clone()
        };
        println!(
            "{:<20} {:<10} {:<30} {}",
            server.name, server.transport, target_display, desc
        );
    }
    Ok(())
}
