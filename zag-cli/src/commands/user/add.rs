//! Handler for `zag user add`.

use anyhow::Result;
use zag_serve::user::UserStore;

pub(crate) fn run(username: String, home_dir: String, password: Option<String>) -> Result<()> {
    let password = match password {
        Some(p) => p,
        None => {
            eprint!("Password: ");
            let p = rpassword::read_password()?;
            eprint!("Confirm password: ");
            let p2 = rpassword::read_password()?;
            if p != p2 {
                anyhow::bail!("Passwords do not match");
            }
            p
        }
    };

    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    let mut store = UserStore::load()?;
    store.add_user(&username, &password, &home_dir)?;

    // Create the home directory and per-user log directory
    let home_path = std::path::Path::new(&home_dir);
    if !home_path.exists() {
        std::fs::create_dir_all(home_path)?;
        eprintln!("Created home directory: {home_dir}");
    }

    let logs_dir = UserStore::user_logs_dir(&username);
    std::fs::create_dir_all(&logs_dir)?;

    eprintln!("User '{username}' created successfully.");
    eprintln!("Home directory: {home_dir}");
    eprintln!("Logs directory: {}", logs_dir.display());
    Ok(())
}
