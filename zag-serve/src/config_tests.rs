use crate::config::{ConnectConfig, ServeConfig};
use crate::generate_token;

#[test]
fn generate_token_is_64_hex_chars() {
    let token = generate_token();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn generate_token_is_unique() {
    let t1 = generate_token();
    let t2 = generate_token();
    assert_ne!(t1, t2);
}

#[test]
fn serve_config_defaults() {
    let config = ServeConfig::default();
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 2100);
    assert!(config.server.token.is_none());
    assert!(config.server.tls_cert.is_none());
    assert!(config.server.tls_key.is_none());
    assert!(!config.server.force_sandbox);
}

#[test]
fn serve_config_force_sandbox_from_toml() {
    let toml_str = r#"
[server]
host = "0.0.0.0"
port = 2100
force_sandbox = true
"#;
    let config: ServeConfig = toml::from_str(toml_str).unwrap();
    assert!(config.server.force_sandbox);
}

#[test]
fn serve_config_force_sandbox_defaults_false() {
    let toml_str = r#"
[server]
host = "0.0.0.0"
port = 2100
"#;
    let config: ServeConfig = toml::from_str(toml_str).unwrap();
    assert!(!config.server.force_sandbox);
}

#[test]
fn connect_config_round_trip() {
    let config = ConnectConfig {
        url: "https://example.com:2100".to_string(),
        token: "abc123".to_string(),
        username: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ConnectConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.url, config.url);
    assert_eq!(parsed.token, config.token);
}
