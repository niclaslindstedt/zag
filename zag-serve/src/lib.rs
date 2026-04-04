//! Network server for zag — remote access to AI agent orchestration.
//!
//! Provides an HTTP/WebSocket server that exposes zag's orchestration API
//! over the network, enabling remote clients (e.g., mobile apps) to spawn
//! and monitor agent sessions on a host machine.

pub mod auth;
pub mod config;
pub mod handlers;
pub mod router;
pub mod types;
pub mod ws;

use anyhow::Result;
use log::info;
use std::net::SocketAddr;

use auth::ServerState;
use config::ServeConfig;

/// Parameters for starting the server.
pub struct ServerParams {
    pub host: String,
    pub port: u16,
    pub token: String,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

/// Generate a cryptographically random 32-byte hex token.
pub fn generate_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Start the zag server. This function blocks until the server is shut down.
pub async fn start_server(params: ServerParams) -> Result<()> {
    let state = ServerState {
        token: params.token,
    };

    let app = router::build_router(state);
    let addr: SocketAddr = format!("{}:{}", params.host, params.port).parse()?;

    if let (Some(cert_path), Some(key_path)) = (&params.tls_cert, &params.tls_key) {
        info!("Starting zag server with TLS on {}", addr);

        let tls_config =
            axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;

        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        info!("Starting zag server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

/// Save a token to the serve config.
pub fn save_token_to_config(token: &str) -> Result<()> {
    let mut config = ServeConfig::load();
    config.server.token = Some(token.to_string());
    config.save()
}
