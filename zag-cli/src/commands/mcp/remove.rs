use anyhow::Result;
use zag::mcp;

pub(crate) fn run(name: &str, root: Option<&str>) -> Result<()> {
    mcp::remove_server(name, root)?;
    println!(
        "\x1b[32m✓\x1b[0m Removed MCP server '{}' and cleaned up provider configs.",
        name
    );
    Ok(())
}
