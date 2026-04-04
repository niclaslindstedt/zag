//! Handler for `zag serve` — start the HTTP/WebSocket server.

use anyhow::{Result, bail};

pub(crate) struct ServeParams {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
    pub generate_token: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

pub(crate) async fn run_serve(params: ServeParams) -> Result<()> {
    // Validate TLS args
    if params.tls_cert.is_some() != params.tls_key.is_some() {
        bail!("Both --tls-cert and --tls-key must be provided together");
    }

    // Resolve token: flag > env > config > generate
    let token = if let Some(t) = params.token {
        t
    } else if let Ok(t) = std::env::var("ZAG_SERVE_TOKEN") {
        t
    } else {
        let config = zag_serve::config::ServeConfig::load();
        if let Some(t) = config.server.token {
            t
        } else if params.generate_token {
            let t = zag_serve::generate_token();
            zag_serve::save_token_to_config(&t)?;
            log::info!("Generated token: {}", t);
            eprintln!("Generated token: {}", t);
            t
        } else {
            bail!(
                "No auth token provided. Use --token, ZAG_SERVE_TOKEN env var, \
                 or --generate-token to create one."
            );
        }
    };

    let scheme = if params.tls_cert.is_some() {
        "https"
    } else {
        "http"
    };
    eprintln!(
        "Starting zag server on {}://{}:{}",
        scheme, params.host, params.port
    );

    zag_serve::start_server(zag_serve::ServerParams {
        host: params.host,
        port: params.port,
        token,
        tls_cert: params.tls_cert,
        tls_key: params.tls_key,
    })
    .await
}
