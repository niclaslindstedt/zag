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

/// Check if the remote server is reachable. Returns true if healthy, false if unreachable.
/// Uses a file-based cache to avoid checking on every command invocation.
pub(crate) async fn check_server_health(config: &ConnectConfig) -> bool {
    // Check cache first — skip the network call if we checked recently
    if ConnectConfig::is_health_cache_valid(30) {
        return true;
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok();

    let Some(client) = client else { return false };

    let health_url = format!("{}/api/v1/health", config.url);
    match client.get(&health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let _ = ConnectConfig::update_health_cache();
            true
        }
        _ => false,
    }
}

/// Proxy a command to the remote server.
pub(crate) async fn proxy_command(config: &ConnectConfig, command: &Commands) -> Result<()> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    match command {
        Commands::User { command: sub, json } => proxy_user(&client, config, sub, *json).await,
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
                &format!("/api/v1/sessions/{session_id}/status"),
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
                params.push(format!("type={t}"));
            }
            if let Some(n) = last {
                params.push(format!("last={n}"));
            }
            if let Some(s) = after_seq {
                params.push(format!("after_seq={s}"));
            }
            if let Some(s) = before_seq {
                params.push(format!("before_seq={s}"));
            }
            let qs = if params.is_empty() {
                String::new()
            } else {
                format!("?{}", params.join("&"))
            };
            proxy_get_json(
                &client,
                config,
                &format!("/api/v1/sessions/{session_id}/events{qs}"),
                *json,
            )
            .await
        }
        Commands::Spawn {
            prompt,
            plan: _,
            agent,
            metadata,
            json,
            depends_on,
            inject_context,
            timeout,
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
                "timeout": timeout,
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
                    &format!("/api/v1/sessions/{id}/cancel"),
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
                    &format!("/api/v1/sessions/{id}/output"),
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
                    &format!("/api/v1/sessions/{id}/stream"),
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
                params.push(format!("tag={t}"));
            }
            if let Some(t) = event_type {
                params.push(format!("type={t}"));
            }
            let qs = if params.is_empty() {
                String::new()
            } else {
                format!("?{}", params.join("&"))
            };
            proxy_ws_stream(
                &config.url,
                &config.token,
                &format!("/api/v1/subscribe{qs}"),
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
                    &format!("/api/v1/sessions/{id}/input"),
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
        Commands::Summary {
            session_ids,
            tag,
            stats,
            json,
            ..
        } => {
            let body = serde_json::json!({
                "session_ids": session_ids,
                "tag": tag,
                "stats": stats,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/summary", &body, *json).await
        }
        Commands::Retry {
            session_ids,
            tag,
            failed,
            model,
            json,
            ..
        } => {
            let body = serde_json::json!({
                "session_ids": session_ids,
                "tag": tag,
                "failed": failed,
                "model": model,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/retry", &body, *json).await
        }
        Commands::Gc {
            force,
            older_than,
            keep_logs,
            json,
            ..
        } => {
            let body = serde_json::json!({
                "force": force,
                "older_than": older_than,
                "keep_logs": keep_logs,
            });
            proxy_post_json(&client, config, "/api/v1/gc", &body, *json).await
        }
        Commands::Log {
            message,
            session,
            level,
            data,
            ..
        } => {
            let session_id = match session.as_deref() {
                Some(id) => id.to_string(),
                None => match std::env::var("ZAG_SESSION_ID") {
                    Ok(id) => id,
                    Err(_) => bail!("Remote log requires --session or ZAG_SESSION_ID"),
                },
            };
            let body = serde_json::json!({
                "message": message,
                "level": level,
                "data": data,
            });
            proxy_post_json(
                &client,
                config,
                &format!("/api/v1/sessions/{session_id}/log"),
                &body,
                true,
            )
            .await
        }
        Commands::Env {
            session_id,
            root: _,
            ..
        } => {
            let id = session_id.as_deref().unwrap_or("latest");
            proxy_get_json(&client, config, &format!("/api/v1/sessions/{id}/env"), true).await
        }
        Commands::Search {
            query,
            regex,
            case_sensitive,
            provider,
            role,
            tool,
            tool_kind,
            from,
            to,
            session,
            tag,
            global,
            json,
            count,
            limit,
            ..
        } => {
            let tool_kind_str = tool_kind.as_ref().map(|k| format!("{k:?}").to_lowercase());
            let body = serde_json::json!({
                "query": query,
                "regex": regex,
                "case_sensitive": case_sensitive,
                "provider": provider,
                "role": role,
                "tool": tool,
                "tool_kind": tool_kind_str,
                "from": from,
                "to": to,
                "session": session,
                "tag": tag,
                "global": global,
                "count": count,
                "limit": limit,
            });
            proxy_post_json(&client, config, "/api/v1/search", &body, *json).await
        }
        Commands::Pipe {
            session_ids,
            tag,
            prompt,
            agent,
            output: _,
            json,
        } => {
            let body = serde_json::json!({
                "session_ids": session_ids,
                "tag": tag,
                "prompt": prompt,
                "provider": agent.provider,
                "model": agent.model,
                "root": agent.root,
                "auto_approve": agent.auto_approve,
                "system_prompt": agent.system_prompt,
                "add_dirs": if agent.add_dirs.is_empty() { None } else { Some(&agent.add_dirs) },
                "size": agent.size,
                "max_turns": agent.max_turns,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/pipe", &body, *json).await
        }
        Commands::Config { args, root } => {
            let body = serde_json::json!({
                "args": args,
                "root": root,
            });
            proxy_post_json(&client, config, "/api/v1/config", &body, true).await
        }
        Commands::Capability {
            provider,
            format,
            pretty,
            ..
        } => {
            let mut params = vec![];
            if let Some(p) = provider {
                params.push(format!("provider={p}"));
            }
            params.push(format!("format={format}"));
            if *pretty {
                params.push("pretty=true".to_string());
            }
            let qs = if params.is_empty() {
                String::new()
            } else {
                format!("?{}", params.join("&"))
            };
            proxy_get_json(&client, config, &format!("/api/v1/capability{qs}"), true).await
        }
        Commands::Discover {
            provider,
            models,
            resolve,
            json,
            format,
            pretty,
            ..
        } => {
            let mut params = vec![];
            if let Some(p) = provider {
                params.push(format!("provider={p}"));
            }
            if *models {
                params.push("models=true".to_string());
            }
            if let Some(r) = resolve {
                params.push(format!("resolve={r}"));
            }
            if *json {
                params.push("json=true".to_string());
            }
            if let Some(f) = format {
                params.push(format!("format={f}"));
            }
            if *pretty {
                params.push("pretty=true".to_string());
            }
            let qs = if params.is_empty() {
                String::new()
            } else {
                format!("?{}", params.join("&"))
            };
            proxy_get_json(&client, config, &format!("/api/v1/discover{qs}"), true).await
        }
        Commands::Skills { command: sub, json } => {
            let body = proxy_skills_body(sub);
            proxy_post_json(&client, config, "/api/v1/skills", &body, *json).await
        }
        Commands::Mcp {
            command: sub,
            json,
            root,
        } => {
            let body = proxy_mcp_body(sub, root.as_deref());
            proxy_post_json(&client, config, "/api/v1/mcp", &body, *json).await
        }
        Commands::Broadcast {
            message,
            tag,
            global,
            ..
        } => {
            let msg = match message {
                Some(m) => m.clone(),
                None => bail!("Remote broadcast requires a message argument"),
            };
            let body = serde_json::json!({
                "message": msg,
                "tag": tag,
                "global": global,
            });
            proxy_post_json(&client, config, "/api/v1/sessions/broadcast", &body, true).await
        }
        Commands::Review {
            uncommitted,
            base,
            commit,
            title,
            prompt,
            agent,
        } => {
            let body = serde_json::json!({
                "uncommitted": uncommitted,
                "base": base,
                "commit": commit,
                "title": title,
                "prompt": prompt,
                "provider": agent.provider,
                "model": agent.model,
                "root": agent.root,
                "auto_approve": agent.auto_approve,
                "add_dirs": if agent.add_dirs.is_empty() { None } else { Some(&agent.add_dirs) },
            });
            proxy_post_json(&client, config, "/api/v1/review", &body, true).await
        }
        Commands::Watch {
            session_id,
            tag,
            latest,
            on_event,
            filter_expr,
            once,
            json,
            command: cmd,
            ..
        } => {
            proxy_watch(
                config,
                session_id.as_deref(),
                tag.as_deref(),
                *latest,
                on_event,
                filter_expr.as_deref(),
                *once,
                *json,
                cmd,
            )
            .await
        }
        Commands::Whoami { json } => proxy_get_json(&client, config, "/api/v1/health", *json).await,
        Commands::Man { .. } => {
            bail!("Man pages are available locally. Use `zag disconnect` to view them.");
        }
        _ => {
            bail!(
                "This command requires an interactive terminal and cannot run in remote mode. \
                 Use `zag disconnect` to run locally."
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
        bail!("Spawn failed ({status}): {body}");
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
        bail!("Wait failed ({status}): {body}");
    }

    // 3. Get output
    let url = format!("{}/api/v1/sessions/{}/output", config.url, session_id);
    let resp = client.get(&url).bearer_auth(&config.token).send().await?;

    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        bail!("Output failed ({status}): {body}");
    }

    // Print based on output format
    let is_json = matches!(output_format, Some("json" | "json-pretty"));
    if is_json {
        println!("{body}");
    } else {
        // Extract just the result text for plain-text output
        let output: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(result) = output["result"].as_str() {
            println!("{result}");
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
        bail!("Server error ({status}): {body}");
    }

    println!("{body}");
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
        bail!("Server error ({status}): {body}");
    }

    println!("{body}");
    Ok(())
}

async fn proxy_ws_stream(base_url: &str, token: &str, path: &str, _json: bool) -> Result<()> {
    use futures_util::StreamExt;

    // Convert http(s) URL to ws(s) URL
    let ws_url = base_url
        .replace("https://", "wss://")
        .replace("http://", "ws://");
    let url = format!("{ws_url}{path}");

    // Build request with auth header
    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(&url)
        .header("Authorization", format!("Bearer {token}"))
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
        .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {e}"))?;

    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                println!("{text}");
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Err(e) => {
                bail!("WebSocket error: {e}");
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
            proxy_get_json(client, config, &format!("/api/v1/sessions/{id}"), json).await
        }
        SessionCommand::Delete { id } => {
            proxy_delete(client, config, &format!("/api/v1/sessions/{id}")).await
        }
        SessionCommand::Update {
            id,
            name,
            description,
            tags,
            clear_tags,
        } => {
            let body = serde_json::json!({
                "name": name,
                "description": description,
                "tags": if tags.is_empty() { None } else { Some(tags) },
                "clear_tags": clear_tags,
            });
            proxy_patch(client, config, &format!("/api/v1/sessions/{id}"), &body).await
        }
        SessionCommand::Import => {
            let url = format!("{}/api/v1/sessions/import", config.url);
            let resp = client
                .post(&url)
                .bearer_auth(&config.token)
                .json(&serde_json::json!({}))
                .send()
                .await?;

            let status = resp.status();
            let body = resp.text().await?;

            if !status.is_success() {
                bail!("Server error ({status}): {body}");
            }

            if json {
                println!("{body}");
            } else {
                let imported = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("imported").and_then(|n| n.as_u64()))
                    .unwrap_or(0);
                println!("Imported {imported} historical session log(s)");
            }
            Ok(())
        }
    }
}

async fn proxy_ps(
    client: &reqwest::Client,
    config: &ConnectConfig,
    command: &Option<zag_orch::ps::PsCommand>,
    json: bool,
) -> Result<()> {
    use zag_orch::ps::PsCommand;
    match command {
        None | Some(PsCommand::List { .. }) => {
            proxy_get_json(client, config, "/api/v1/processes", json).await
        }
        Some(PsCommand::Show { id }) => {
            proxy_get_json(client, config, &format!("/api/v1/processes/{id}"), json).await
        }
        Some(PsCommand::Stop { id }) => {
            let body = serde_json::json!({});
            proxy_post_json(
                client,
                config,
                &format!("/api/v1/processes/{id}/stop"),
                &body,
                json,
            )
            .await
        }
        Some(PsCommand::Kill { id }) => {
            let body = serde_json::json!({});
            proxy_post_json(
                client,
                config,
                &format!("/api/v1/processes/{id}/kill"),
                &body,
                json,
            )
            .await
        }
    }
}

async fn proxy_user(
    client: &reqwest::Client,
    config: &ConnectConfig,
    command: &crate::cli::UserCommand,
    json: bool,
) -> Result<()> {
    use crate::cli::UserCommand;
    match command {
        UserCommand::Add {
            username,
            home_dir,
            password,
        } => {
            let password = match password {
                Some(p) => p.clone(),
                None => {
                    eprint!("Password: ");
                    let p = rpassword::read_password()?;
                    eprint!("Confirm password: ");
                    let p2 = rpassword::read_password()?;
                    if p != p2 {
                        bail!("Passwords do not match");
                    }
                    p
                }
            };
            if password.is_empty() {
                bail!("Password cannot be empty");
            }
            let body = serde_json::json!({
                "username": username,
                "password": password,
                "home_dir": home_dir,
            });
            proxy_post_json(client, config, "/api/v1/users/add", &body, json).await
        }
        UserCommand::Remove { username } => {
            let body = serde_json::json!({ "username": username });
            proxy_post_json(client, config, "/api/v1/users/remove", &body, json).await
        }
        UserCommand::List => proxy_get_json(client, config, "/api/v1/users", json).await,
        UserCommand::Passwd { username, password } => {
            let password = match password {
                Some(p) => p.clone(),
                None => {
                    eprint!("New password: ");
                    let p = rpassword::read_password()?;
                    eprint!("Confirm password: ");
                    let p2 = rpassword::read_password()?;
                    if p != p2 {
                        bail!("Passwords do not match");
                    }
                    p
                }
            };
            if password.is_empty() {
                bail!("Password cannot be empty");
            }
            let body = serde_json::json!({
                "username": username,
                "password": password,
            });
            proxy_post_json(client, config, "/api/v1/users/passwd", &body, json).await
        }
    }
}

async fn proxy_delete(client: &reqwest::Client, config: &ConnectConfig, path: &str) -> Result<()> {
    let url = format!("{}{}", config.url, path);
    let resp = client
        .delete(&url)
        .bearer_auth(&config.token)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Server error ({status}): {body}");
    }

    println!("{body}");
    Ok(())
}

async fn proxy_patch(
    client: &reqwest::Client,
    config: &ConnectConfig,
    path: &str,
    body: &serde_json::Value,
) -> Result<()> {
    let url = format!("{}{}", config.url, path);
    let resp = client
        .patch(&url)
        .bearer_auth(&config.token)
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        bail!("Server error ({status}): {body}");
    }

    println!("{body}");
    Ok(())
}

fn proxy_skills_body(sub: &crate::cli::SkillsCommand) -> serde_json::Value {
    use crate::cli::SkillsCommand;
    match sub {
        SkillsCommand::List => serde_json::json!({ "command": "list" }),
        SkillsCommand::Show { name } => serde_json::json!({
            "command": "show",
            "name": name,
        }),
        SkillsCommand::Add { name, description } => serde_json::json!({
            "command": "add",
            "name": name,
            "description": description,
        }),
        SkillsCommand::Remove { name } => serde_json::json!({
            "command": "remove",
            "name": name,
        }),
        SkillsCommand::Sync { provider } => serde_json::json!({
            "command": "sync",
            "from": provider,
        }),
        SkillsCommand::Import { from } => serde_json::json!({
            "command": "import",
            "from": from,
        }),
    }
}

fn proxy_mcp_body(sub: &crate::cli::McpCommand, root: Option<&str>) -> serde_json::Value {
    use crate::cli::McpCommand;
    match sub {
        McpCommand::List => serde_json::json!({ "command": "list", "root": root }),
        McpCommand::Show { name } => serde_json::json!({
            "command": "show",
            "name": name,
            "root": root,
        }),
        McpCommand::Add {
            name,
            transport,
            command,
            args,
            url,
            env,
            description,
            global,
        } => serde_json::json!({
            "command": "add",
            "name": name,
            "transport": transport,
            "server_command": command,
            "args": args,
            "url": url,
            "env": env,
            "description": description,
            "global": global,
            "root": root,
        }),
        McpCommand::Remove { name } => serde_json::json!({
            "command": "remove",
            "name": name,
            "root": root,
        }),
        McpCommand::Sync { provider } => serde_json::json!({
            "command": "sync",
            "from": provider,
            "root": root,
        }),
        McpCommand::Import { from } => serde_json::json!({
            "command": "import",
            "from": from,
            "root": root,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
async fn proxy_watch(
    config: &ConnectConfig,
    session_id: Option<&str>,
    tag: Option<&str>,
    _latest: bool,
    on_event: &str,
    filter_expr: Option<&str>,
    once: bool,
    json: bool,
    command: &[String],
) -> Result<()> {
    use futures_util::StreamExt;

    // Build subscribe WebSocket path
    let mut params = vec![format!("type={}", on_event)];
    if let Some(t) = tag {
        params.push(format!("tag={t}"));
    }

    let path = if let Some(id) = session_id {
        format!("/api/v1/sessions/{}/stream?{}", id, params.join("&"))
    } else {
        format!("/api/v1/subscribe?{}", params.join("&"))
    };

    let ws_url = config
        .url
        .replace("https://", "wss://")
        .replace("http://", "ws://");
    let url = format!("{ws_url}{path}");

    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(&url)
        .header("Authorization", format!("Bearer {}", config.token))
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
        .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {e}"))?;

    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                // Apply filter if specified
                if let Some(filter) = filter_expr {
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(&text) {
                        if !matches_filter(&event, filter) {
                            continue;
                        }
                    }
                }

                if json {
                    println!("{text}");
                }

                // Execute command if specified
                if !command.is_empty() {
                    let status = std::process::Command::new(&command[0])
                        .args(&command[1..])
                        .env("ZAG_EVENT", text.as_ref() as &str)
                        .status();
                    if let Err(e) = status {
                        eprintln!("Failed to execute command: {e}");
                    }
                }

                if once {
                    break;
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Err(e) => {
                bail!("WebSocket error: {e}");
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "proxy_tests.rs"]
mod tests;

/// Check if a JSON event matches a simple "key=value,key=value" filter expression.
fn matches_filter(event: &serde_json::Value, filter: &str) -> bool {
    for pair in filter.split(',') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match event.get(key) {
                Some(serde_json::Value::String(s)) if s == value => {}
                Some(v) if v.to_string().trim_matches('"') == value => {}
                _ => return false,
            }
        }
    }
    true
}
