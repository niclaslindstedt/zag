//! Handler for `zag connect` — connect to a remote zag server.

use anyhow::{Result, bail};

/// Normalize a server URL: strip trailing slashes and prepend https:// if no scheme given.
pub(crate) fn normalize_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if url.starts_with("https://") || url.starts_with("http://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

pub(crate) async fn run_connect(
    url: String,
    token: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let url = normalize_url(&url);

    // Validate connectivity by hitting the health endpoint
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // allow self-signed certs for local networks
        .build()?;

    let health_url = format!("{url}/api/v1/health");
    match client.get(&health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Connected to {url}");
        }
        Ok(resp) => {
            bail!(
                "Server at {} returned status {}. Is this a zag server?",
                url,
                resp.status()
            );
        }
        Err(e) => {
            bail!("Cannot reach server at {url}: {e}");
        }
    }

    // Authenticate via user account or legacy token
    if let Some(ref user) = username {
        // User-account mode: login with username/password
        let password = match password {
            Some(p) => p,
            None => {
                eprint!("Password: ");
                rpassword::read_password()?
            }
        };

        let login_url = format!("{url}/api/v1/login");
        let resp = client
            .post(&login_url)
            .json(&serde_json::json!({
                "username": user,
                "password": password,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Login failed ({status}): {body}");
        }

        let login_resp: serde_json::Value = resp.json().await?;
        let session_token = login_resp["token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Server did not return a token"))?;
        let home_dir = login_resp["home_dir"].as_str().unwrap_or("");

        let config = zag_serve::config::ConnectConfig {
            url: url.clone(),
            token: session_token.to_string(),
            username: Some(user.clone()),
        };
        config.save()?;

        eprintln!("Logged in as '{user}' to {url}");
        eprintln!("Home directory: {home_dir}");
        eprintln!("All zag commands will now proxy through the remote server.");
        eprintln!("Use `zag disconnect` to return to local mode.");
    } else {
        // Legacy token mode
        let token = if let Some(t) = token {
            t
        } else if let Ok(t) = std::env::var("ZAG_CONNECT_TOKEN") {
            t
        } else {
            bail!("No auth provided. Use --token, --username, or set ZAG_CONNECT_TOKEN env var.");
        };

        let config = zag_serve::config::ConnectConfig {
            url: url.clone(),
            token,
            username: None,
        };
        config.save()?;

        eprintln!("Connected to {url}");
        eprintln!("All zag commands will now proxy through the remote server.");
        eprintln!("Use `zag disconnect` to return to local mode.");
    }

    Ok(())
}

pub(crate) fn run_disconnect() -> Result<()> {
    if !zag_serve::config::ConnectConfig::is_connected() {
        eprintln!("Not connected to any remote server.");
        return Ok(());
    }

    zag_serve::config::ConnectConfig::remove()?;
    eprintln!("Disconnected from remote server.");
    Ok(())
}

#[cfg(test)]
#[path = "connect_tests.rs"]
mod tests;
