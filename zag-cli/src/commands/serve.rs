//! Handler for `zag serve` — start the HTTPS/WebSocket server.

use anyhow::{Result, bail};

pub(crate) struct ServeParams {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub token: Option<String>,
    pub generate_token: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
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
    let token = if let Some(t) = params.token {
        t
    } else if let Ok(t) = std::env::var("ZAG_SERVE_TOKEN") {
        t
    } else {
        if let Some(t) = config.server.token.clone() {
            t
        } else if params.generate_token {
            let t = zag_serve::generate_token();
            zag_serve::save_token_to_config(&t)?;
            log::info!("Generated token: {}", t);
            eprintln!("Generated token: {}", t);
            t
        } else {
            let t = zag_serve::generate_token();
            zag_serve::save_token_to_config(&t)?;
            log::info!("Auto-generated token: {}", t);
            eprintln!("Auto-generated token: {}", t);
            t
        }
    };

    eprintln!("Starting zag server on https://{}:{}", host, port);

    zag_serve::start_server(zag_serve::ServerParams {
        host,
        port,
        token,
        tls_cert,
        tls_key,
    })
    .await
}
