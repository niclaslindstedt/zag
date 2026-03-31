use anyhow::{Result, bail};
use zag::config::Config;
use zag::{mcp, session, skills};

use crate::cli::{McpCommand, SessionCommand, SkillsCommand};
use crate::session_log;

/// Handle `zag config` subcommand.
pub(crate) fn run_config(args: Vec<String>, root: Option<&str>) -> Result<()> {
    if args.is_empty() {
        // Print full config file contents
        let path = Config::config_path(root);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            print!("{}", content);
        } else {
            println!("No config file found at {}", path.display());
            println!("Run any agent command to create a default config.");
        }
        return Ok(());
    }

    // Handle special subcommands: init, path, get
    if args.len() == 1 && args[0] == "init" {
        let created = Config::init(root)?;
        let path = Config::config_path(root);
        if created {
            println!("Created default config at {}", path.display());
        } else {
            println!("Config already exists at {}", path.display());
        }
        return Ok(());
    }

    if args.len() == 1 && args[0] == "reset" {
        let path = Config::config_path(root);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Config::init(root)?;
        println!("Config reset to defaults at {}", path.display());
        return Ok(());
    }

    if args.len() == 1 && args[0] == "path" {
        let path = Config::config_path(root);
        println!("{}", path.display());
        return Ok(());
    }

    if args.len() == 2 && args[0] == "unset" {
        let mut config = Config::load(root).unwrap_or_default();
        config.unset_value(&args[1])?;
        config.save(root)?;
        println!("{} (unset)", args[1]);
        return Ok(());
    }

    if args.len() == 1 && args[0] == "list" {
        let config = Config::load(root).unwrap_or_default();
        println!("{:<25} VALUE", "KEY");
        println!("{}", "-".repeat(55));
        for key in Config::VALID_KEYS {
            let value = config
                .get_value(key)
                .unwrap_or_else(|| "(not set)".to_string());
            println!("{:<25} {}", key, value);
        }
        return Ok(());
    }

    // Explicit `config get <key>` syntax
    if args.len() == 2 && args[0] == "get" {
        let config = Config::load(root).unwrap_or_default();
        match config.get_value(&args[1]) {
            Some(val) => println!("{}", val),
            None => println!("(not set)"),
        }
        return Ok(());
    }

    // Parse key=value or key value
    let (key, value) = if args.len() == 1 {
        // Single arg — check for key=value, otherwise treat as key lookup
        if let Some((k, v)) = args[0].split_once('=') {
            (k.to_string(), v.to_string())
        } else {
            // Implicit get: `config <key>` reads the value
            let config = Config::load(root).unwrap_or_default();
            match config.get_value(&args[0]) {
                Some(val) => println!("{}", val),
                None => println!("(not set)"),
            }
            return Ok(());
        }
    } else {
        // Two args: key value
        (args[0].clone(), args[1].clone())
    };

    let mut config = Config::load(root).unwrap_or_default();
    config.set_value(&key, &value)?;
    config.save(root)?;
    println!("{} = {}", key, value);
    Ok(())
}

pub(crate) fn run_session(command: SessionCommand, json: bool, root: Option<&str>) -> Result<()> {
    match command {
        SessionCommand::List {
            provider,
            limit,
            global,
            name,
            tag,
        } => {
            let store = if global {
                session::SessionStore::load_all()?
            } else {
                session::SessionStore::load(root)?
            };
            let mut sessions = store.list();
            if let Some(ref p) = provider {
                sessions.retain(|s| s.provider == *p);
            }
            if let Some(ref n) = name {
                let n_lower = n.to_lowercase();
                sessions.retain(|s| {
                    s.name
                        .as_ref()
                        .map(|sn| sn.to_lowercase().contains(&n_lower))
                        .unwrap_or(false)
                });
            }
            if let Some(ref t) = tag {
                let t_lower = t.to_lowercase();
                sessions.retain(|s| s.tags.iter().any(|st| st.to_lowercase() == t_lower));
            }
            if let Some(n) = limit {
                sessions.truncate(n);
            }
            if json {
                println!("{}", serde_json::to_string(&sessions)?);
                return Ok(());
            }
            if sessions.is_empty() {
                println!("No sessions found.");
                return Ok(());
            }
            println!(
                "{:<38} {:<20} {:<10} {:<12} CREATED",
                "SESSION ID", "NAME", "PROVIDER", "MODEL"
            );
            println!("{}", "-".repeat(110));
            for s in &sessions {
                let name_display = s
                    .name
                    .as_deref()
                    .map(|n| {
                        if n.len() > 18 {
                            format!("{}…", &n[..17])
                        } else {
                            n.to_string()
                        }
                    })
                    .unwrap_or_else(|| "-".to_string());
                println!(
                    "{:<38} {:<20} {:<10} {:<12} {}",
                    s.session_id, name_display, s.provider, s.model, s.created_at
                );
            }
        }
        SessionCommand::Show { id } => {
            let store = session::SessionStore::load(root)?;
            match store.get(&id) {
                Some(info) => {
                    if json {
                        println!("{}", serde_json::to_string(&info)?);
                        return Ok(());
                    }
                    println!("Session ID:          {}", info.session_id);
                    println!("Provider:            {}", info.provider);
                    println!("Model:               {}", info.model);
                    println!("Created:             {}", info.created_at);
                    if let Some(ref name) = info.name {
                        println!("Name:                {}", name);
                    }
                    if let Some(ref desc) = info.description {
                        println!("Description:         {}", desc);
                    }
                    if !info.tags.is_empty() {
                        println!("Tags:                {}", info.tags.join(", "));
                    }
                    if let Some(ref pid) = info.provider_session_id {
                        println!("Provider Session ID: {}", pid);
                    }
                    if let Some(ref wp) = info.worktree_path {
                        println!("Worktree:            {}", wp);
                    }
                    if let Some(ref sb) = info.sandbox_name {
                        println!("Sandbox:             {}", sb);
                    }
                    println!("Log Completeness:    {}", info.log_completeness);
                }
                None => {
                    bail!("Session not found: {}", id);
                }
            }
        }
        SessionCommand::Import => {
            let imported = session_log::run_default_backfill(root)?;
            println!("Imported {} historical session log(s)", imported);
        }
        SessionCommand::Delete { id } => {
            let mut store = session::SessionStore::load(root)?;
            if store.get(&id).is_none() {
                bail!("Session not found: {}", id);
            }
            store.remove(&id);
            store.save(root)?;
            if json {
                println!(r#"{{"deleted":"{}"}}"#, id);
            } else {
                println!("Deleted session: {}", id);
            }
        }
        SessionCommand::Update {
            id,
            name,
            description,
            tags,
            clear_tags,
        } => {
            let mut store = session::SessionStore::load(root)?;
            let entry = store.sessions.iter_mut().find(|e| e.session_id == id);
            let entry = match entry {
                Some(e) => e,
                None => bail!("Session not found: {}", id),
            };
            if name.is_some() {
                entry.name = name;
            }
            if description.is_some() {
                entry.description = description;
            }
            if clear_tags {
                entry.tags.clear();
            }
            if !tags.is_empty() {
                entry.tags.extend(tags);
            }
            let updated = session::SessionInfo::from(&*entry);
            store.save(root)?;
            if json {
                println!("{}", serde_json::to_string(&updated)?);
            } else {
                println!("Updated session: {}", id);
            }
        }
    }
    Ok(())
}

pub(crate) fn run_skills(command: SkillsCommand, json: bool) -> Result<()> {
    match command {
        SkillsCommand::List => {
            let skill_list = skills::list_skills()?;
            if json {
                println!("{}", serde_json::to_string(&skill_list)?);
                return Ok(());
            }
            if skill_list.is_empty() {
                println!("No skills found in {}", skills::skills_dir().display());
                println!("Use 'agent skills add <name>' to create one.");
                return Ok(());
            }
            println!("{:<20} {:<50} PATH", "NAME", "DESCRIPTION");
            println!("{}", "-".repeat(100));
            for skill in &skill_list {
                println!(
                    "{:<20} {:<50} {}",
                    skill.name,
                    if skill.description.len() > 48 {
                        format!("{}...", &skill.description[..48])
                    } else {
                        skill.description.clone()
                    },
                    skill.dir.display()
                );
            }
        }
        SkillsCommand::Show { name } => {
            let skill = skills::get_skill(&name)?;
            if json {
                println!("{}", serde_json::to_string(&skill)?);
                return Ok(());
            }
            println!("Name:        {}", skill.name);
            println!("Description: {}", skill.description);
            println!("Path:        {}", skill.dir.display());
            if !skill.body.is_empty() {
                println!();
                println!("{}", skill.body);
            }
        }
        SkillsCommand::Add { name, description } => {
            let description = description.unwrap_or_default();
            let dir = skills::add_skill(&name, &description)?;
            println!(
                "\x1b[32m✓\x1b[0m Created skill '{}' at {}",
                name,
                dir.display()
            );
            println!(
                "Edit {} to add your skill content.",
                dir.join("SKILL.md").display()
            );
        }
        SkillsCommand::Remove { name } => {
            skills::remove_skill(&name)?;
            println!(
                "\x1b[32m✓\x1b[0m Removed skill '{}' and its provider symlinks.",
                name
            );
        }
        SkillsCommand::Sync { provider } => {
            let skill_list = skills::load_all_skills()?;
            if skill_list.is_empty() {
                println!("No skills to sync.");
                return Ok(());
            }
            let providers: Vec<&str> = if let Some(ref p) = provider {
                vec![p.as_str()]
            } else {
                vec!["claude", "gemini", "copilot", "codex"]
            };
            for p in providers {
                if skills::provider_skills_dir(p).is_some() {
                    skills::sync_skills_for_provider(p, &skill_list)?;
                    println!(
                        "\x1b[32m✓\x1b[0m Synced {} skill(s) for {}",
                        skill_list.len(),
                        p
                    );
                } else {
                    println!("  {} does not support native skills (skipped)", p);
                }
            }
        }
        SkillsCommand::Import { from } => {
            let imported = skills::import_skills(&from)?;
            if imported.is_empty() {
                println!("No new skills to import from '{}'.", from);
            } else {
                for name in &imported {
                    println!("\x1b[32m✓\x1b[0m Imported skill '{}'", name);
                }
                println!("Imported {} skill(s) from '{}'.", imported.len(), from);
            }
        }
    }
    Ok(())
}

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
