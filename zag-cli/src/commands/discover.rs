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
            print_provider_detail(&cap);
        } else {
            println!("{}", capability::format_capability(&cap, fmt, pretty)?);
        }
    } else {
        let caps = get_all_capabilities();
        if fmt == "text" {
            print_summary_table(&caps);
        } else {
            println!("{}", capability::format_capabilities(&caps, fmt, pretty)?);
        }
    }

    Ok(())
}

fn print_summary_table(caps: &[ProviderCapability]) {
    println!(
        "{:<10} {:<28} {:>6}  {:<6} {:<6} {:<7}",
        "PROVIDER", "DEFAULT MODEL", "MODELS", "RESUME", "JSON", "LOGS"
    );
    println!("{}", "-".repeat(70));
    for cap in caps {
        let resume = if cap.features.resume.supported {
            "yes"
        } else {
            "no"
        };
        let json_out = if cap.features.json_output.supported {
            "yes"
        } else {
            "no"
        };
        let logs = cap
            .features
            .session_logs
            .completeness
            .as_deref()
            .unwrap_or("-");
        println!(
            "{:<10} {:<28} {:>6}  {:<6} {:<6} {:<7}",
            cap.provider,
            cap.default_model,
            cap.available_models.len(),
            resume,
            json_out,
            logs,
        );
    }
}

fn print_provider_detail(cap: &ProviderCapability) {
    println!("Provider: {}", cap.provider);
    println!("Default model: {}", cap.default_model);
    println!(
        "Size mappings: small={}, medium={}, large={}",
        cap.size_mappings.small, cap.size_mappings.medium, cap.size_mappings.large
    );
    println!("Available models:");
    for m in &cap.available_models {
        println!("  - {m}");
    }
    println!();
    println!("Features:");
    print_feature("  interactive", &cap.features.interactive);
    print_feature("  non-interactive", &cap.features.non_interactive);
    print_feature("  resume", &cap.features.resume);
    print_feature("  resume-with-prompt", &cap.features.resume_with_prompt);
    print_session_log("  session-logs", &cap.features.session_logs);
    print_feature("  json-output", &cap.features.json_output);
    print_feature("  stream-json", &cap.features.stream_json);
    print_feature("  json-schema", &cap.features.json_schema);
    print_feature("  input-format", &cap.features.input_format);
    print_streaming_input("  streaming-input", &cap.features.streaming_input);
    print_feature("  worktree", &cap.features.worktree);
    print_feature("  sandbox", &cap.features.sandbox);
    print_feature("  system-prompt", &cap.features.system_prompt);
    print_feature("  auto-approve", &cap.features.auto_approve);
    print_feature("  review", &cap.features.review);
    print_feature("  add-dirs", &cap.features.add_dirs);
    print_feature("  max-turns", &cap.features.max_turns);
}

fn print_feature(label: &str, f: &capability::FeatureSupport) {
    let status = if f.supported {
        if f.native { "native" } else { "wrapper" }
    } else {
        "no"
    };
    println!("{label:<24} {status}");
}

fn print_streaming_input(label: &str, f: &capability::StreamingInputSupport) {
    let status = if f.supported {
        let base = if f.native { "native" } else { "wrapper" };
        match f.semantics.as_deref() {
            Some(s) => format!("{base} ({s})"),
            None => base.to_string(),
        }
    } else {
        "no".to_string()
    };
    println!("{label:<24} {status}");
}

fn print_session_log(label: &str, f: &capability::SessionLogSupport) {
    let status = if f.supported {
        match f.completeness.as_deref() {
            Some(c) => {
                if f.native {
                    c.to_string()
                } else {
                    format!("{c} (wrapper)")
                }
            }
            None => "yes".to_string(),
        }
    } else {
        "no".to_string()
    };
    println!("{label:<24} {status}");
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
        let caps = get_all_capabilities();
        if format == "text" {
            for cap in &caps {
                println!("{}:", cap.provider);
                for m in &cap.available_models {
                    println!("  {m}");
                }
            }
        } else {
            println!("{}", capability::format_models(&caps, format, pretty)?);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
