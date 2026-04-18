#[cfg(test)]
#[path = "broadcast_tests.rs"]
mod tests;

use anyhow::{Result, bail};
use log::debug;
use zag_orch::messaging;

pub(crate) struct BroadcastParams {
    pub message: Option<String>,
    pub tag: Option<String>,
    pub global: bool,
    pub output: Option<String>,
    pub root: Option<String>,
    pub quiet: bool,
    pub raw: bool,
}

pub(crate) async fn run_broadcast(params: BroadcastParams) -> Result<()> {
    let BroadcastParams {
        message,
        tag,
        global,
        output,
        root,
        quiet,
        raw,
    } = params;

    let session_ids =
        messaging::resolve_broadcast_session_ids(tag.as_deref(), global, root.as_deref())?;

    debug!(
        "Broadcast: resolved {} session(s){}",
        session_ids.len(),
        tag.as_ref()
            .map(|t| format!(" for tag '{t}'"))
            .unwrap_or_default()
    );

    let msg = if let Some(m) = message {
        m
    } else {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            bail!("No message provided. Pass a message argument or pipe to stdin.");
        }
        trimmed
    };

    let msg = messaging::maybe_wrap_message(&msg, raw);
    let broadcast_result = messaging::send_broadcast(&session_ids, &msg, root.as_deref()).await?;

    let output_json = matches!(output.as_deref(), Some("json") | Some("json-pretty"));
    if output_json {
        let results: Vec<serde_json::Value> = broadcast_result
            .outcomes
            .iter()
            .map(|o| match &o.result {
                Ok(()) => serde_json::json!({"session_id": o.session_id, "status": "sent"}),
                Err(e) => serde_json::json!({
                    "session_id": o.session_id,
                    "status": "failed",
                    "error": e,
                }),
            })
            .collect();
        let result = serde_json::json!({
            "results": results,
            "summary": {
                "sent": broadcast_result.sent(),
                "failed": broadcast_result.failed(),
                "total": broadcast_result.total(),
            }
        });
        if output.as_deref() == Some("json-pretty") {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("{}", serde_json::to_string(&result)?);
        }
    } else if !quiet {
        let sent = broadcast_result.sent();
        eprintln!(
            "> Sent to {sent} session{} ({} failed)",
            if sent == 1 { "" } else { "s" },
            broadcast_result.failed(),
        );
    }

    Ok(())
}
