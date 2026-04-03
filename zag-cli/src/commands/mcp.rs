use anyhow::{Result, bail};
use zag::mcp;

use crate::cli::McpCommand;

pub(crate) fn run_mcp(command: McpCommand, json: bool, root: Option<&str>) -> Result<()> {
    match command {
        McpCommand::List => {
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
        }
        McpCommand::Show { name } => {
            let server = mcp::get_server(&name, root)?;
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
                println!("Command:     {}", cmd);
            }
            if !server.args.is_empty() {
                println!("Args:        {}", server.args.join(" "));
            }
            if let Some(ref url) = server.url {
                println!("URL:         {}", url);
            }
            if let Some(ref var) = server.bearer_token_env_var {
                println!("Bearer Env:  {}", var);
            }
            if !server.env.is_empty() {
                println!("Env:");
                for (k, v) in &server.env {
                    println!("  {} = {}", k, v);
                }
            }
            if !server.headers.is_empty() {
                println!("Headers:");
                for (k, v) in &server.headers {
                    println!("  {}: {}", k, v);
                }
            }
        }
        McpCommand::Add {
            name,
            transport,
            command,
            args,
            url,
            env,
            description,
            global,
        } => {
            let project = !global;
            let mut env_map = std::collections::BTreeMap::new();
            for pair in &env {
                if let Some((k, v)) = pair.split_once('=') {
                    env_map.insert(k.to_string(), v.to_string());
                } else {
                    bail!("Invalid --env format '{}'. Expected KEY=VALUE", pair);
                }
            }
            let server = mcp::McpServer {
                name: name.clone(),
                description: description.unwrap_or_default(),
                transport: transport.clone(),
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
        }
        McpCommand::Remove { name } => {
            mcp::remove_server(&name, root)?;
            println!(
                "\x1b[32m✓\x1b[0m Removed MCP server '{}' and cleaned up provider configs.",
                name
            );
        }
        McpCommand::Sync { provider } => {
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
        }
        McpCommand::Import { from } => {
            let imported = mcp::import_servers(&from)?;
            if imported.is_empty() {
                println!("No new MCP servers to import from '{}'.", from);
            } else {
                for name in &imported {
                    println!("\x1b[32m✓\x1b[0m Imported MCP server '{}'", name);
                }
                println!("Imported {} MCP server(s) from '{}'.", imported.len(), from);
            }
        }
    }
    Ok(())
}
