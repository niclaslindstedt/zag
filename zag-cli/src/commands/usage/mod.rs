mod cancel;
mod list;

use anyhow::Result;

use crate::cli::UsageCommand;

pub(crate) fn run_usage(command: UsageCommand, json: bool, root: Option<&str>) -> Result<()> {
    match command {
        UsageCommand::List => list::run(json, root),
        UsageCommand::Cancel { incident_id } => cancel::run(&incident_id, json, root),
    }
}
