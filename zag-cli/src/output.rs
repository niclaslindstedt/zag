pub use zag_agent::output::*;

use anyhow::Result;
use log::{debug, info};

/// Print agent output in the requested format.
pub(crate) fn print_agent_output(
    agent_out: &AgentOutput,
    output_fmt: Option<&str>,
    show_usage: bool,
    verbose: bool,
) -> Result<()> {
    match output_fmt {
        Some("json") => {
            let json = serde_json::to_string(agent_out)?;
            println!("{}", json);
        }
        Some("json-pretty") => {
            let json = serde_json::to_string_pretty(agent_out)?;
            println!("{}", json);
        }
        Some("stream-json") => {
            for event in &agent_out.events {
                let json = serde_json::to_string(event)?;
                println!("{}", json);
            }
        }
        _ => {
            process_agent_output(agent_out, show_usage, verbose)?;
        }
    }
    Ok(())
}

/// Process and display structured agent output.
pub(crate) fn process_agent_output(
    output: &AgentOutput,
    show_usage: bool,
    verbose: bool,
) -> Result<()> {
    // Show decorations only when verbose is enabled (or not in quiet mode for non-exec paths)
    let show_decorations = verbose && !crate::logging::is_quiet();

    if show_decorations {
        let min_level = LogLevel::Info;

        let log_entries = output.to_log_entries(min_level);
        for entry in log_entries {
            match entry.level {
                LogLevel::Debug => debug!("{}", entry.message),
                LogLevel::Info => info!("{}", entry.message),
                LogLevel::Warn => log::warn!("{}", entry.message),
                LogLevel::Error => log::error!("{}", entry.message),
            }
        }

        for event in &output.events {
            if let Event::ToolExecution {
                tool_name, result, ..
            } = event
            {
                if result.success {
                    info!("✓ Tool '{}' executed successfully", tool_name);
                } else {
                    log::warn!(
                        "✗ Tool '{}' failed: {}",
                        tool_name,
                        result.error.as_deref().unwrap_or("unknown error")
                    );
                }
            }
        }
    }

    // Display final result if available (always shown)
    if let Some(result) = output.final_result() {
        if show_decorations {
            println!("\n{}", result);
        } else {
            println!("{}", result);
        }
    }

    if show_decorations {
        if let Some(cost) = output.total_cost_usd {
            info!("Total cost: ${:.4}", cost);
        }

        if show_usage && let Some(usage) = &output.usage {
            info!(
                "Token usage - Input: {}, Output: {}",
                usage.input_tokens, usage.output_tokens
            );

            if let Some(cache_read) = usage.cache_read_tokens
                && cache_read > 0
            {
                info!("Cache read: {} tokens", cache_read);
            }

            if let Some(cache_creation) = usage.cache_creation_tokens
                && cache_creation > 0
            {
                info!("Cache created: {} tokens", cache_creation);
            }

            if let Some(web_search) = usage.web_search_requests
                && web_search > 0
            {
                info!("Web search requests: {}", web_search);
            }

            if let Some(web_fetch) = usage.web_fetch_requests
                && web_fetch > 0
            {
                info!("Web fetch requests: {}", web_fetch);
            }
        }
    }

    Ok(())
}
