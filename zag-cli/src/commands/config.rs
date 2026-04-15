use anyhow::Result;
use zag_agent::config::Config;

/// Handle `zag config` subcommand.
pub(crate) fn run_config(args: Vec<String>, root: Option<&str>) -> Result<()> {
    if args.is_empty() {
        // Print full config file contents
        let path = Config::config_path(root);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            print!("{content}");
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
            println!("{key:<25} {value}");
        }
        return Ok(());
    }

    // Explicit `config get <key>` syntax
    if args.len() == 2 && args[0] == "get" {
        let config = Config::load(root).unwrap_or_default();
        match config.get_value(&args[1]) {
            Some(val) => println!("{val}"),
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
                Some(val) => println!("{val}"),
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
    println!("{key} = {value}");
    Ok(())
}
