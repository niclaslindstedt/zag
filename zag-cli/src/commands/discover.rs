use anyhow::Result;
use zag_agent::capability::{self, ProviderCapability, get_all_capabilities, get_capability};

pub(crate) fn run_discover(
    provider: Option<&str>,
    models_only: bool,
    resolve: Option<&str>,
    json: bool,
    format: Option<&str>,
    pretty: bool,
) -> Result<()> {
    // --resolve: resolve a model alias
    if let Some(model_input) = resolve {
        let provider = provider.ok_or_else(|| {
            anyhow::anyhow!(
                "--resolve requires --provider (-p) to specify which provider to resolve for"
            )
        })?;
        let rm = capability::resolve_model(provider, model_input)?;
        let fmt = format.unwrap_or(if json { "json" } else { "text" });
        if fmt == "text" {
            if rm.is_alias {
                println!("{} -> {}", rm.input, rm.resolved);
            } else {
                println!("{} (not an alias, passes through unchanged)", rm.resolved);
            }
        } else {
            println!("{}", capability::format_resolved_model(&rm, fmt, pretty)?);
        }
        return Ok(());
    }

    let fmt = format.unwrap_or(if json { "json" } else { "text" });

    // --models: list models
    if models_only {
        return print_models(provider, fmt, pretty);
    }

    // Default: provider summary
    if let Some(p) = provider {
        let cap = get_capability(p)?;
        if fmt == "text" {
            print!("{}", capability::format_provider_detail(&cap));
        } else {
            println!("{}", capability::format_capability(&cap, fmt, pretty)?);
        }
    } else {
        let caps = get_all_capabilities();
        if fmt == "text" {
            print!("{}", capability::format_summary_table(&caps));
        } else {
            println!("{}", capability::format_capabilities(&caps, fmt, pretty)?);
        }
    }

    Ok(())
}

fn print_models(provider: Option<&str>, format: &str, pretty: bool) -> Result<()> {
    if let Some(p) = provider {
        let cap = get_capability(p)?;
        if format == "text" {
            for m in &cap.available_models {
                println!("{m}");
            }
        } else {
            println!("{}", capability::format_models(&[cap], format, pretty)?);
        }
    } else {
        let caps: Vec<ProviderCapability> = get_all_capabilities();
        if format == "text" {
            print!("{}", capability::format_models_text(&caps));
        } else {
            println!("{}", capability::format_models(&caps, format, pretty)?);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
