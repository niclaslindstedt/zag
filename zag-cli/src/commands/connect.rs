//! Handler for `zag connect` — connect to a remote zag server.

use anyhow::{Result, bail};

pub(crate) async fn run_connect(url: String, token: Option<String>) -> Result<()> {
    let token = if let Some(t) = token {
        t
    } else if let Ok(t) = std::env::var("ZAG_CONNECT_TOKEN") {
        t
    } else {
        bail!("No auth token provided. Use --token or set ZAG_CONNECT_TOKEN env var.");
    };

    // Normalize URL (strip trailing slash)
    let url = url.trim_end_matches('/').to_string();

    // Validate connectivity by hitting the health endpoint
    let health_url = format!("{}/api/v1/health", url);
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // allow self-signed certs for local networks
        .build()?;

    match client.get(&health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            log::info!("Connected to {}", url);
        }
        Ok(resp) => {
            bail!(
                "Server at {} returned status {}. Is this a zag server?",
                url,
                resp.status()
            );
        }
        Err(e) => {
            bail!("Cannot reach server at {}: {}", url, e);
        }
    }

    // Save connection config
    let config = zag_serve::config::ConnectConfig {
        url: url.clone(),
        token,
    };
    config.save()?;

    eprintln!("Connected to {}", url);
    eprintln!("All zag commands will now proxy through the remote server.");
    eprintln!("Use `zag disconnect` to return to local mode.");

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
