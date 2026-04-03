mod add;
mod import;
mod list;
mod remove;
mod show;
mod sync;

use anyhow::Result;

use crate::cli::SkillsCommand;

pub(crate) fn run_skills(command: SkillsCommand, json: bool) -> Result<()> {
    match command {
        SkillsCommand::List => list::run(json),
        SkillsCommand::Show { name } => show::run(&name, json),
        SkillsCommand::Add { name, description } => add::run(&name, description),
        SkillsCommand::Remove { name } => remove::run(&name),
        SkillsCommand::Sync { provider } => sync::run(provider),
        SkillsCommand::Import { from } => import::run(&from),
    }
}
