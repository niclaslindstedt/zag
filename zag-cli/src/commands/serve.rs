//! Handler for `zag serve` — start the HTTPS/WebSocket server.

use anyhow::{Result, bail};

pub(crate) struct ServeParams {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub token: Option<String>,
    pub generate_token: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub force_sandbox: bool,
}

pub(crate) async fn run_serve(params: ServeParams) -> Result<()> {
    // Validate TLS args: if one is provided, both must be
    if params.tls_cert.is_some() != params.tls_key.is_some() {
        bail!("Both --tls-cert and --tls-key must be provided together");
    }

    // Load config once for all fallbacks
    let config = zag_serve::config::ServeConfig::load();

    // Resolve host: flag > config > default (0.0.0.0)
    let host = params.host.unwrap_or(config.server.host.clone());

    // Resolve port: flag > config > default (2100)
    let port = params.port.unwrap_or(config.server.port);

    // Resolve TLS: user-provided flags > config > auto-generated self-signed
    let (tls_cert, tls_key, is_self_signed) =
        if let (Some(cert), Some(key)) = (params.tls_cert, params.tls_key) {
            (cert, key, false)
        } else if let (Some(cert), Some(key)) = (
            config.server.tls_cert.clone(),
            config.server.tls_key.clone(),
        ) {
            (cert, key, false)
        } else {
            let (cert, key) = zag_serve::ensure_self_signed_cert(&host)?;
            (cert, key, true)
        };

    if is_self_signed {
        eprintln!(
            "WARNING: Using auto-generated self-signed certificate. \
             Do not use in production — provide --tls-cert and --tls-key for production deployments."
        );
    }

    // Resolve token: flag > env > config > auto-generate
    // In user-accounts mode (users.json exists) the legacy token is optional.
    let has_user_accounts = zag_serve::user::UserStore::exists();
    let token = if let Some(t) = params.token {
        Some(t)
    } else if let Ok(t) = std::env::var("ZAG_SERVE_TOKEN") {
        Some(t)
    } else {
        match config.server.token {
            Some(t) if !params.generate_token => Some(t),
            _ => {
                if has_user_accounts {
                    // No legacy token needed in user-accounts mode
                    None
                } else {
                    let t = zag_serve::generate_token();
                    zag_serve::save_token_to_config(&t)?;
                    Some(t)
                }
            }
        }
    };

    // Resolve force_sandbox: flag > config
    let force_sandbox = params.force_sandbox || config.server.force_sandbox;

    if has_user_accounts {
        eprintln!(
            "User accounts mode: loaded from {}",
            zag_serve::user::UserStore::path().display()
        );
    }
    if let Some(ref t) = token {
        eprintln!("Token: {}", t);
    }
    if force_sandbox {
        eprintln!("Force sandbox: enabled (all connected users run in Docker sandboxes)");
    }
    eprintln!("Starting zag server on https://{}:{}", host, port);

    zag_serve::start_server(zag_serve::ServerParams {
        host,
        port,
        token,
        tls_cert,
        tls_key,
        force_sandbox,
    })
    .await
}
