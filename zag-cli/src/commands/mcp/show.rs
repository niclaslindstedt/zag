use anyhow::Result;
use zag_agent::mcp;

pub(crate) fn run(name: &str, json: bool, root: Option<&str>) -> Result<()> {
    let server = mcp::get_server(name, root)?;
    if json {
        println!("{}", serde_json::to_string(&server)?);
        return Ok(());
    }
    println!("Name:        {}", server.name);
    println!("Transport:   {}", server.transport);
    if !server.description.is_empty() {
        println!("Description: {}", server.description);
    }
    if let Some(ref cmd) = server.command {
        println!("Command:     {cmd}");
    }
    if !server.args.is_empty() {
        println!("Args:        {}", server.args.join(" "));
    }
    if let Some(ref url) = server.url {
        println!("URL:         {url}");
    }
    if let Some(ref var) = server.bearer_token_env_var {
        println!("Bearer Env:  {var}");
    }
    if !server.env.is_empty() {
        println!("Env:");
        for (k, v) in &server.env {
            println!("  {k} = {v}");
        }
    }
    if !server.headers.is_empty() {
        println!("Headers:");
        for (k, v) in &server.headers {
            println!("  {k}: {v}");
        }
    }
    Ok(())
}
