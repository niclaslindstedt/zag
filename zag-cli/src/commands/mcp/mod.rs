mod add;
mod import;
mod list;
mod remove;
mod show;
mod sync;

use anyhow::Result;

use crate::cli::McpCommand;

pub(crate) fn run_mcp(command: McpCommand, json: bool, root: Option<&str>) -> Result<()> {
    match command {
        McpCommand::List => list::run(json, root),
        McpCommand::Show { name } => show::run(&name, json, root),
        McpCommand::Add {
            name,
            transport,
            command,
            args,
            url,
            env,
            description,
            global,
        } => add::run(
            &name,
            &transport,
            command,
            args,
            url,
            env,
            description,
            global,
            root,
        ),
        McpCommand::Remove { name } => remove::run(&name, root),
        McpCommand::Sync { provider } => sync::run(provider, root),
        McpCommand::Import { from } => import::run(&from),
    }
}
