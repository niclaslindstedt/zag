//! User account management commands.

mod add;
mod list;
mod passwd;
mod remove;

use anyhow::Result;

use crate::cli::UserCommand;

pub(crate) fn run_user(command: UserCommand, json: bool) -> Result<()> {
    match command {
        UserCommand::Add {
            username,
            home_dir,
            password,
        } => add::run(username, home_dir, password),
        UserCommand::Remove { username } => remove::run(username),
        UserCommand::List => list::run(json),
        UserCommand::Passwd { username, password } => passwd::run(username, password),
    }
}
