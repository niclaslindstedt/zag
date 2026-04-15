//! Handler for `zag user remove`.

use anyhow::Result;
use zag_serve::user::UserStore;

pub(crate) fn run(username: String) -> Result<()> {
    let mut store = UserStore::load()?;
    store.remove_user(&username)?;
    eprintln!("User '{username}' removed.");
    eprintln!("Note: home directory and logs were not deleted.");
    Ok(())
}
