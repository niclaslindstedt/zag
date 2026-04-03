use anyhow::Result;
use zag_agent::mcp;

pub(crate) fn run(provider: Option<String>, root: Option<&str>) -> Result<()> {
    let servers = mcp::load_all_servers(root)?;
    if servers.is_empty() {
        println!("No MCP servers to sync.");
        return Ok(());
    }
    let providers: Vec<&str> = if let Some(ref p) = provider {
        vec![p.as_str()]
    } else {
        mcp::MCP_PROVIDERS.to_vec()
    };
    for p in providers {
        match mcp::sync_servers_for_provider(p, &servers) {
            Ok(synced) => {
                println!("\x1b[32m✓\x1b[0m Synced {} MCP server(s) for {}", synced, p);
            }
            Err(e) => {
                println!("  Failed to sync for {}: {}", p, e);
            }
        }
    }
    Ok(())
}
