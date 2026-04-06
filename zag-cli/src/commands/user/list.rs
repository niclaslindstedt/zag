//! Handler for `zag user list`.

use anyhow::Result;
use zag_serve::user::UserStore;

pub(crate) fn run(json: bool) -> Result<()> {
    let store = UserStore::load()?;
    let users = store.list_users();

    if json {
        let output: Vec<serde_json::Value> = users
            .iter()
            .map(|u| {
                serde_json::json!({
                    "username": u.username,
                    "home_dir": u.home_dir,
                    "enabled": u.enabled,
                    "created_at": u.created_at,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if users.is_empty() {
        eprintln!("No users configured. Use `zag user add` to create one.");
    } else {
        for user in users {
            let status = if user.enabled { "enabled" } else { "disabled" };
            println!(
                "{:<20} {:<40} {} ({})",
                user.username, user.home_dir, user.created_at, status
            );
        }
    }
    Ok(())
}
