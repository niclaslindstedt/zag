use anyhow::Result;
use zag::mcp;

pub(crate) fn run(from: &str) -> Result<()> {
    let imported = mcp::import_servers(from)?;
    if imported.is_empty() {
        println!("No new MCP servers to import from '{}'.", from);
    } else {
        for name in &imported {
            println!("\x1b[32m✓\x1b[0m Imported MCP server '{}'", name);
        }
        println!("Imported {} MCP server(s) from '{}'.", imported.len(), from);
    }
    Ok(())
}
