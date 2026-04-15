//! Network server for zag — remote access to AI agent orchestration.
//!
//! Provides an HTTPS/WebSocket server that exposes zag's orchestration API
//! over the network, enabling remote clients (e.g., mobile apps) to spawn
//! and monitor agent sessions on a host machine.

pub mod auth;
pub mod config;
pub mod handlers;
pub mod middleware;
pub mod router;
pub mod session_token;
pub mod types;
pub mod user;
pub mod ws;

use anyhow::Result;
use log::info;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use auth::ServerState;
use session_token::TokenStore;
use user::UserStore;

/// Parameters for starting the server.
pub struct ServerParams {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
    pub tls_cert: String,
    pub tls_key: String,
    /// When true, all connected users' agent sessions are forced to run inside a Docker sandbox.
    pub force_sandbox: bool,
}

/// Generate a cryptographically random 32-byte hex token.
pub fn generate_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Directory for auto-generated TLS files.
fn tls_dir() -> PathBuf {
    zag_agent::config::Config::global_base_dir().join("tls")
}

/// Generate a self-signed TLS certificate and save it to ~/.zag/tls/.
/// If certificates already exist on disk, loads and returns those instead.
/// Returns (cert_path, key_path).
pub fn ensure_self_signed_cert(host: &str) -> Result<(String, String)> {
    let dir = tls_dir();
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        return Ok((
            cert_path.to_string_lossy().into_owned(),
            key_path.to_string_lossy().into_owned(),
        ));
    }

    std::fs::create_dir_all(&dir)?;

    let mut params = rcgen::CertificateParams::new(vec!["localhost".to_string()])?;
    params
        .subject_alt_names
        .push(rcgen::SanType::DnsName("localhost".try_into()?));
    if host != "localhost" && host != "0.0.0.0" && host != "127.0.0.1" {
        if let Ok(dns) = host.try_into() {
            params.subject_alt_names.push(rcgen::SanType::DnsName(dns));
        }
    }
    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(
            std::net::Ipv4Addr::new(127, 0, 0, 1),
        )));
    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(
            std::net::Ipv4Addr::new(0, 0, 0, 0),
        )));

    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key_pair.serialize_pem())?;

    Ok((
        cert_path.to_string_lossy().into_owned(),
        key_path.to_string_lossy().into_owned(),
    ))
}

/// Start the zag server. This function blocks until the server is shut down.
pub async fn start_server(params: ServerParams) -> Result<()> {
    // Determine auth mode: user accounts (if users.json exists) or legacy token
    let (user_store, token_store) = if UserStore::exists() {
        let store = UserStore::load()?;
        info!(
            "User accounts mode: loaded {} user(s) from {}",
            store.users.len(),
            UserStore::path().display()
        );
        (
            Some(Arc::new(store)),
            Some(Arc::new(RwLock::new(TokenStore::new()))),
        )
    } else {
        (None, None)
    };

    let state = ServerState {
        token: params.token,
        user_store,
        token_store,
        force_sandbox: params.force_sandbox,
    };

    let app = router::build_router(state);
    let addr: SocketAddr = format!("{}:{}", params.host, params.port).parse()?;

    info!("Starting zag server with TLS on {addr}");

    let tls_config =
        axum_server::tls_rustls::RustlsConfig::from_pem_file(&params.tls_cert, &params.tls_key)
            .await?;

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// Save a token to the serve config.
pub fn save_token_to_config(token: &str) -> Result<()> {
    let mut config = config::ServeConfig::load();
    config.server.token = Some(token.to_string());
    config.save()
}
