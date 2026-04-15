//! Handler for `zag user passwd`.

use anyhow::Result;
use zag_serve::user::UserStore;

pub(crate) fn run(username: String, password: Option<String>) -> Result<()> {
    let password = match password {
        Some(p) => p,
        None => {
            eprint!("New password: ");
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
    store.change_password(&username, &password)?;
    eprintln!("Password changed for user '{username}'.");
    Ok(())
}
