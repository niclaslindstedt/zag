mod delete;
mod import;
mod list;
mod show;
mod update;

use anyhow::Result;

use crate::cli::SessionCommand;

pub(crate) fn run_session(command: SessionCommand, json: bool, root: Option<&str>) -> Result<()> {
    match command {
        SessionCommand::List {
            provider,
            limit,
            global,
            name,
            tag,
            parent,
        } => list::run(provider, limit, global, name, tag, parent, json, root),
        SessionCommand::Show { id } => show::run(&id, json, root),
        SessionCommand::Import => import::run(root),
        SessionCommand::Delete { id } => delete::run(&id, json, root),
        SessionCommand::Update {
            id,
            name,
            description,
            tags,
            clear_tags,
        } => update::run(&id, name, description, tags, clear_tags, json, root),
    }
}
