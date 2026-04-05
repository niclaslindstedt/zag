//! Transparent proxy: when connected to a remote server, route commands through it.

use anyhow::{Result, bail};
use zag_serve::config::ConnectConfig;

use crate::cli::Commands;

/// Check if we're connected to a remote server and the command should be proxied.
/// Returns None if not connected or command should run locally.
pub(crate) fn should_proxy(command: &Commands) -> Option<ConnectConfig> {
    // These commands always run locally
    match command {
        Commands::Connect { .. }
        | Commands::Disconnect
        | Commands::Serve { .. }
        | Commands::Relay { .. } => return None,
        _ => {}
    }

    ConnectConfig::load()
}

/// Proxy a command to the remote server.
pub(crate) async fn proxy_command(config: &ConnectConfig, command: &Commands) -> Result<()> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    match command {
        Commands::Session {
            command: sub,
            json,
            root: _,
        } => proxy_session(&client, config, sub, *json).await,
        Commands::Ps { json, command: sub } => proxy_ps(&client, config, sub, *json).await,
        Commands::Status {
            session_id,
            json,
            root: _,
        } => {
            proxy_get_json(
                &client,
                config,
                &format!("/api/v1/sessions/{}/status", session_id),
                *json,
            )
            .await
        }
        Commands::Events {
            session_id,
            event_type,
            last,
            after_seq,
            before_seq,
            count: _,
            json,
            root: _,
        } => {
            let mut params = vec![];
            if let Some(t) = event_type {
                params.push(format!("type={}", t));
            }
            if let Some(n) = last {
                params.push(format!("last={}", n));
            }
            if let Some(s) = after_seq {
                params.push(format!("after_seq={}", s));
            }
            if let Some(s) = before_seq {
                params.push(format!("before_seq={}", s));
            }
            let qs = if params.is_empty() {
                String::new()
            } else {
                format!("?{}", params.join("&"))
            };
            proxy_get_json(
                &client,
                config,
                &format!("/api/v1/sessions/{}/events{}", session_id, qs),
                *json,
            )
            .await
        }
        Commands::Spawn {
            prompt,
            agent,
            metadata,
            json,
            depends_on,
            inject_context,
            interactive,
        } => {
            let body = serde_json::json!({
                "prompt": prompt,
                "provider": agent.provider,
                "model": agent.model,
                "root": agent.root,
                "auto_approve": agent.auto_approve,
                "system_prompt": agent.system_prompt,
                "add_dirs": if agent.add_dirs.is_empty() { None } else { Some(&agent.add_dirs) },
                "size": agent.size,
                "max_turns": agent.max_turns,
                "name": metadata.name,
                "description": metadata.description,
                "tags": if metadata.tags.is_empty() { None } else { Some(&metadata.tags) },
                "depends_on": if depends_on.is_empty() { None } else { Some(depends_on) },
                "inject_context": inject_context,
                "interactive": interactive,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/spawn", &body, *json).await
        }
        Commands::Cancel {
            session_ids,
            tag: _,
            reason,
            json,
            root: _,
        } => {
            for id in session_ids {
                let body = serde_json::json!({ "reason": reason });
                proxy_post_json(
                    &client,
                    config,
                    &format!("/api/v1/sessions/{}/cancel", id),
                    &body,
                    *json,
                )
                .await?;
            }
            Ok(())
        }
        Commands::Collect {
            session_ids,
            tag,
            json,
            root: _,
        } => {
            let body = serde_json::json!({
                "session_ids": session_ids,
                "tag": tag,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/collect", &body, *json).await
        }
        Commands::Wait {
            session_ids,
            tag,
            latest: _,
            timeout,
            any,
            json,
            root: _,
        } => {
            let body = serde_json::json!({
                "session_ids": session_ids,
                "tag": tag,
                "timeout": timeout,
                "any": any,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/wait", &body, *json).await
        }
        Commands::Output {
            session_id,
            latest: _,
            output_name: _,
            tag: _,
            json,
            root: _,
        } => {
            if let Some(id) = session_id {
                proxy_get_json(
                    &client,
                    config,
                    &format!("/api/v1/sessions/{}/output", id),
                    *json,
                )
                .await
            } else {
                bail!("Remote output requires a session ID");
            }
        }
        Commands::Listen {
            session_id,
            latest: _,
            active: _,
            ps: _,
            json,
            ..
        } => {
            if let Some(id) = session_id {
                proxy_ws_stream(
                    &config.url,
                    &config.token,
                    &format!("/api/v1/sessions/{}/stream", id),
                    *json,
                )
                .await
            } else {
                bail!("Remote listen requires a session ID");
            }
        }
        Commands::Subscribe {
            tag,
            event_type,
            global: _,
            json,
            root: _,
        } => {
            let mut params = vec![];
            if let Some(t) = tag {
                params.push(format!("tag={}", t));
            }
            if let Some(t) = event_type {
                params.push(format!("type={}", t));
            }
            let qs = if params.is_empty() {
                String::new()
            } else {
                format!("?{}", params.join("&"))
            };
            proxy_ws_stream(
                &config.url,
                &config.token,
                &format!("/api/v1/subscribe{}", qs),
                *json,
            )
            .await
        }
        Commands::Input {
            session, message, ..
        } => {
            if let (Some(id), Some(msg)) = (session, message) {
                let body = serde_json::json!({ "message": msg });
                proxy_post_json(
                    &client,
                    config,
                    &format!("/api/v1/sessions/{}/input", id),
                    &body,
                    true,
                )
                .await
            } else {
                bail!("Remote input requires --session and a message");
            }
        }
        Commands::Exec {
            prompt,
            agent,
            metadata,
            output,
            ..
        } => proxy_exec(&client, config, prompt, agent, metadata, output.as_deref()).await,
        Commands::Whoami { json } => proxy_get_json(&client, config, "/api/v1/health", *json).await,
        _ => {
            bail!(
                "This command is not supported in remote mode. Use `zag disconnect` to run locally."
            );
        }
    }
}

/// Exec in remote mode: spawn a session, wait for it, then print its output.
async fn proxy_exec(
    client: &reqwest::Client,
    config: &ConnectConfig,
    prompt: &str,
    agent: &crate::cli::AgentArgs,
    metadata: &crate::cli::SessionMetadataArgs,
    output_format: Option<&str>,
) -> Result<()> {
    // 1. Spawn
    let spawn_body = serde_json::json!({
        "prompt": prompt,
        "provider": agent.provider,
        "model": agent.model,
        "root": agent.root,
        "auto_approve": agent.auto_approve,
        "system_prompt": agent.system_prompt,
        "add_dirs": if agent.add_dirs.is_empty() { None } else { Some(&agent.add_dirs) },
        "size": agent.size,
        "max_turns": agent.max_turns,
        "name": metadata.name,
        "description": metadata.description,
        "tags": if metadata.tags.is_empty() { None } else { Some(&metadata.tags) },
    });

    let url = format!("{}/api/v1/sessions/spawn", config.url);
    let resp = client
        .post(&url)
        .bearer_auth(&config.token)
        .json(&spawn_body)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        bail!("Spawn failed ({}): {}", status, body);
    }

    let spawn_resp: serde_json::Value = serde_json::from_str(&body)?;
    let session_id = spawn_resp["session_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No session_id in spawn response"))?;

    // 2. Wait
    let wait_body = serde_json::json!({
        "session_ids": [session_id],
    });
    let url = format!("{}/api/v1/sessions/wait", config.url);
    let resp = client
        .post(&url)
        .bearer_auth(&config.token)
        .json(&wait_body)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        bail!("Wait failed ({}): {}", status, body);
    }

    // 3. Get output
    let url = format!("{}/api/v1/sessions/{}/output", config.url, session_id);
    let resp = client.get(&url).bearer_auth(&config.token).send().await?;

    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        bail!("Output failed ({}): {}", status, body);
    }

    // Print based on output format
    let is_json = matches!(output_format, Some("json" | "json-pretty"));
    if is_json {
        println!("{}", body);
    } else {
        // Extract just the result text for plain-text output
        let output: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(result) = output["result"].as_str() {
            println!("{}", result);
        }
    }

    Ok(())
}

async fn proxy_get_json(
    client: &reqwest::Client,
    config: &ConnectConfig,
    path: &str,
    _json: bool,
) -> Result<()> {
    let url = format!("{}{}", config.url, path);
    let resp = client.get(&url).bearer_auth(&config.token).send().await?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Server error ({}): {}", status, body);
    }

    println!("{}", body);
    Ok(())
}

async fn proxy_post_json(
    client: &reqwest::Client,
    config: &ConnectConfig,
    path: &str,
    body: &serde_json::Value,
    _json: bool,
) -> Result<()> {
    let url = format!("{}{}", config.url, path);
    let resp = client
        .post(&url)
        .bearer_auth(&config.token)
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Server error ({}): {}", status, body);
    }

    println!("{}", body);
    Ok(())
}

async fn proxy_ws_stream(base_url: &str, token: &str, path: &str, _json: bool) -> Result<()> {
    use futures_util::StreamExt;

    // Convert http(s) URL to ws(s) URL
    let ws_url = base_url
        .replace("https://", "wss://")
        .replace("http://", "ws://");
    let url = format!("{}{}", ws_url, path);

    // Build request with auth header
    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header(
            "Host",
            url.split("://")
                .nth(1)
                .unwrap_or("localhost")
                .split('/')
                .next()
                .unwrap_or("localhost"),
        )
        .body(())?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {}", e))?;

    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                println!("{}", text);
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Err(e) => {
                bail!("WebSocket error: {}", e);
            }
            _ => {}
        }
    }

    Ok(())
}

async fn proxy_session(
    client: &reqwest::Client,
    config: &ConnectConfig,
    command: &crate::cli::SessionCommand,
    json: bool,
) -> Result<()> {
    use crate::cli::SessionCommand;
    match command {
        SessionCommand::List { .. } => {
            proxy_get_json(client, config, "/api/v1/sessions", json).await
        }
        SessionCommand::Show { id } => {
            proxy_get_json(client, config, &format!("/api/v1/sessions/{}", id), json).await
        }
        _ => {
            bail!("This session subcommand is not supported in remote mode");
        }
    }
}

async fn proxy_ps(
    client: &reqwest::Client,
    config: &ConnectConfig,
    _command: &Option<zag_orch::ps::PsCommand>,
    json: bool,
) -> Result<()> {
    proxy_get_json(client, config, "/api/v1/processes", json).await
}
