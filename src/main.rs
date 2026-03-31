// Re-export core modules from zag-lib
use zag::agent;
use zag::config;
use zag::factory;
use zag::json_validation;
use zag::sandbox;
use zag::session;
use zag::worktree;

// Re-export provider modules
use zag::providers::claude;
use zag::providers::codex;
use zag::providers::copilot;
use zag::providers::gemini;
use zag::providers::ollama;

// Modules that remain in the binary crate
mod agent_action;
mod broadcast;
mod capability;
mod cleanup;
mod cli;
mod collect;
mod commands;
mod env;
mod input;
mod json_mode;
mod lifecycle;
mod listen;
mod logging;
mod manpage;
mod output;
mod ps;
mod resume;
mod review;
mod search;
mod session_log;
mod session_setup;
mod spawn;
mod status;
mod wait;
mod whoami;

// Re-export from sub-modules so main_tests.rs can use `super::*`
pub(crate) use agent_action::{AgentActionParams, run_agent_action};
pub(crate) use cli::{
    Cli, Commands, SessionIsolationArgs, command_agent_args, command_metadata_args,
    command_session_args, parse_json_schema,
};
pub(crate) use commands::{run_config, run_mcp, run_session, run_skills};

use anyhow::{Result, bail};
use clap::Parser;
use config::Config;
use log::debug;

use broadcast::{BroadcastParams, run_broadcast};
use input::{InputParams, run_input};
use manpage::{HELP_AGENT, print_manpage};
use review::{ReviewParams, run_review};

/// Resolve the provider name from CLI flag, config, or default.
pub(crate) fn resolve_provider(flag: Option<&str>, root: Option<&str>) -> Result<String> {
    if let Some(p) = flag {
        let p = p.to_lowercase();
        if !Config::VALID_PROVIDERS.contains(&p.as_str()) {
            bail!(
                "Invalid provider '{}'. Available: {}",
                p,
                Config::VALID_PROVIDERS.join(", ")
            );
        }
        return Ok(p);
    }

    let config = Config::load(root).unwrap_or_default();
    if let Some(p) = config.provider() {
        return Ok(p.to_string());
    }

    Ok("claude".to_string())
}

/// Capitalize the first letter of a string.
pub(crate) fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle --help-agent before clap parsing so it works without a subcommand.
    if std::env::args().any(|a| a == "--help-agent") {
        print!("{}", HELP_AGENT);
        return Ok(());
    }

    let cli = Cli::parse();

    // In exec mode without --verbose, suppress info-level logging (treat as quiet for the logger)
    let is_exec = matches!(cli.command, Commands::Exec { .. });
    let effective_quiet = cli.quiet || (is_exec && !cli.verbose && !cli.debug);

    // Initialize logging
    logging::init(cli.debug, effective_quiet);
    debug!("Debug logging enabled");

    let quiet = cli.quiet;
    let verbose = cli.verbose;

    // Extract session isolation and metadata args (only present on run/exec)
    let session_args = command_session_args(&cli.command).cloned();
    let metadata_args = command_metadata_args(&cli.command).cloned();
    let json_mode = session_args
        .as_ref()
        .map(|s| s.json || s.json_schema.is_some())
        .unwrap_or(false);
    let json_stream = session_args
        .as_ref()
        .map(|s| s.json_stream)
        .unwrap_or(false);

    // Validate --json-stream is mutually exclusive with --json/--json-schema
    if json_stream && json_mode {
        bail!("--json-stream cannot be combined with --json or --json-schema");
    }

    // Validate --json-stream usage with resume/continue
    if json_stream {
        match &cli.command {
            Commands::Run {
                resume,
                continue_session,
                ..
            } if resume.is_some() || *continue_session => {
                bail!("--json-stream cannot be used with run --resume or run --continue")
            }
            Commands::Run { prompt, .. } if prompt.is_none() => {
                bail!("--json-stream requires a prompt (use exec or run with a prompt)")
            }
            _ => {}
        }
    }

    // Validate --json/--json-schema usage and parse schema once
    let json_schema: Option<serde_json::Value> = if json_mode {
        match &cli.command {
            Commands::Run {
                resume,
                continue_session,
                ..
            } if resume.is_some() || *continue_session => {
                bail!("--json/--json-schema cannot be used with run --resume or run --continue")
            }
            Commands::Run { prompt, .. } if prompt.is_none() => {
                bail!("--json/--json-schema requires a prompt (use exec or run with a prompt)")
            }
            _ => {}
        }

        // Parse and validate schema if provided
        if let Some(ref schema_str) = session_args.as_ref().unwrap().json_schema {
            Some(parse_json_schema(schema_str)?)
        } else {
            None
        }
    } else {
        None
    };

    // Validate --worktree/--sandbox/--session usage with resume/continue
    if let Some(ref sa) = session_args {
        if let Commands::Run {
            resume,
            continue_session,
            ..
        } = &cli.command
        {
            if resume.is_some() || *continue_session {
                if sa.worktree.is_some() {
                    bail!("--worktree cannot be used with run --resume or run --continue");
                }
                if sa.sandbox.is_some() {
                    bail!("--sandbox cannot be used with run --resume or run --continue");
                }
                if sa.session.is_some() {
                    bail!("--session cannot be used with run --resume or run --continue");
                }
            }
        }

        if sa.sandbox.is_some() && sa.worktree.is_some() {
            bail!("--sandbox and --worktree are mutually exclusive");
        }

        // Validate --session is a valid UUID
        if let Some(ref session_id) = sa.session {
            uuid::Uuid::parse_str(session_id).map_err(|_| {
                anyhow::anyhow!("--session must be a valid UUID, got '{}'", session_id)
            })?;
        }
    }

    // Validate auto provider/model usage
    if let Some(agent_args) = command_agent_args(&cli.command) {
        let is_auto_provider = agent_args.provider.as_deref() == Some("auto");
        let is_auto_model = agent_args.model.as_deref() == Some("auto");
        if is_auto_provider || is_auto_model {
            match &cli.command {
                Commands::Review { .. } => bail!("auto cannot be used with review"),
                Commands::Run {
                    resume,
                    continue_session,
                    ..
                } if resume.is_some() || *continue_session => {
                    bail!("auto cannot be used with run --resume or run --continue")
                }
                _ => {}
            }
        }
    }

    match cli.command {
        Commands::Man { command } => {
            debug!("Showing manpage for: {:?}", command);
            print_manpage(command.as_deref())?;
        }
        Commands::Config { args, root } => {
            debug!("Running config subcommand with args: {:?}", args);
            run_config(args, root.as_deref())?;
        }
        Commands::Session {
            command,
            json,
            root,
        } => {
            debug!(
                "Running session subcommand: {:?}",
                std::mem::discriminant(&command)
            );
            run_session(command, json, root.as_deref())?;
        }
        Commands::Skills { command, json } => {
            debug!(
                "Running skills subcommand: {:?}",
                std::mem::discriminant(&command)
            );
            run_skills(command, json)?;
        }
        Commands::Mcp {
            command,
            json,
            root,
        } => {
            debug!(
                "Running mcp subcommand: {:?}",
                std::mem::discriminant(&command)
            );
            run_mcp(command, json, root.as_deref())?;
        }
        Commands::Ps { command, json } => {
            let cmd = command.unwrap_or(ps::PsCommand::List {
                running: false,
                limit: None,
                provider: None,
                children: None,
            });
            ps::run_ps(cmd, json)?;
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
            json: search_json,
            count,
            limit,
            root,
        } => {
            search::run_search_command(
                search::SearchCommandArgs {
                    query,
                    use_regex: regex,
                    case_sensitive,
                    provider,
                    role,
                    tool,
                    tool_kind: tool_kind.map(zag::session_log::ToolKind::from),
                    from,
                    to,
                    session,
                    tag,
                    global,
                    json: search_json,
                    count,
                    limit,
                    root,
                },
                quiet,
            )?;
        }
        Commands::Capability {
            format,
            pretty,
            provider,
            root,
        } => {
            let provider = resolve_provider(provider.as_deref(), root.as_deref())?;
            debug!("Showing capabilities for provider: {}", provider);
            let cap = capability::get_capability(&provider)?;
            let output = capability::format_capability(&cap, &format, pretty)?;
            println!("{}", output);
        }
        Commands::Listen {
            session_id,
            latest,
            active,
            ps,
            json: listen_json,
            text: listen_text,
            rich_text,
            show_thinking,
            timestamps,
            filters,
            root,
        } => {
            let config = Config::load(root.as_deref()).unwrap_or_default();
            let format =
                listen::ListenFormat::from_flags(listen_json, rich_text, listen_text, &config);
            // Resolve --ps to a session_id if provided
            let ps_session_id = ps
                .as_deref()
                .map(listen::resolve_session_from_ps)
                .transpose()?;
            let resolved_session_id = ps_session_id.as_deref().or(session_id.as_deref());
            let log_path =
                listen::resolve_session_log(resolved_session_id, latest, active, root.as_deref())?;
            debug!("Listening to session log: {}", log_path.display());
            let filter_set = if filters.is_empty() {
                None
            } else {
                Some(filters.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>())
            };
            listen::tail_session_log(
                &log_path,
                format,
                show_thinking,
                timestamps,
                &config,
                filter_set.as_deref(),
            )?;
        }
        Commands::Input {
            session,
            message,
            latest,
            active,
            ps,
            input_name,
            global,
            stream,
            output,
            root,
            raw,
        } => {
            run_input(InputParams {
                session,
                message,
                latest,
                active,
                ps,
                input_name,
                global,
                stream,
                output,
                root,
                quiet,
                raw,
            })
            .await?;
        }
        Commands::Broadcast {
            message,
            tag,
            global,
            output,
            root,
            raw,
        } => {
            run_broadcast(BroadcastParams {
                message,
                tag,
                global,
                output,
                root,
                quiet,
                raw,
            })
            .await?;
        }
        Commands::Whoami { json } => {
            whoami::run_whoami(json)?;
        }
        Commands::Status {
            session_id,
            json: status_json,
            root,
        } => {
            status::run_status(&session_id, status_json, root.as_deref())?;
        }
        Commands::Env {
            session_id,
            shell,
            root,
        } => {
            env::run_env(session_id.as_deref(), shell, root.as_deref())?;
        }
        Commands::Collect {
            session_ids,
            tag,
            json: collect_json,
            root,
        } => {
            collect::run_collect(collect::CollectParams {
                session_ids,
                tag,
                json: collect_json,
                root,
            })?;
        }
        Commands::Wait {
            session_ids,
            tag,
            latest,
            timeout,
            any,
            json: wait_json,
            root,
        } => {
            wait::run_wait(wait::WaitParams {
                session_ids,
                tag,
                latest,
                timeout,
                any,
                json: wait_json,
                root,
            })?;
        }
        Commands::Spawn {
            prompt,
            agent,
            metadata,
            json: spawn_json,
        } => {
            let provider = resolve_provider(agent.provider.as_deref(), agent.root.as_deref())?;
            spawn::run_spawn(spawn::SpawnParams {
                prompt,
                provider,
                model: agent.model,
                root: agent.root,
                auto_approve: agent.auto_approve,
                system_prompt: agent.system_prompt,
                add_dirs: agent.add_dirs,
                size: agent.size,
                max_turns: agent.max_turns,
                json: spawn_json,
                metadata: session_setup::SessionMetadata {
                    name: metadata.name,
                    description: metadata.description,
                    tags: metadata.tags,
                },
            })?;
        }
        Commands::Review {
            uncommitted,
            base,
            commit,
            title,
            agent,
        } => {
            run_review(ReviewParams {
                uncommitted,
                base,
                commit,
                title,
                system_prompt: agent.system_prompt,
                model: agent.model,
                root: agent.root,
                auto_approve: agent.auto_approve,
                add_dirs: agent.add_dirs,
                quiet,
            })
            .await?;
        }
        action => {
            let agent_args = command_agent_args(&action).cloned().unwrap();
            let exit_on_failure = matches!(
                &action,
                Commands::Exec {
                    exit_on_failure: true,
                    ..
                }
            );
            let context_session = match &action {
                Commands::Exec { context, .. } => context.clone(),
                Commands::Run { context, .. } => context.clone(),
                _ => None,
            };
            let session_isolation = session_args.unwrap_or(SessionIsolationArgs {
                worktree: None,
                sandbox: None,
                session: None,
                json: false,
                json_schema: None,
                json_stream: false,
            });
            let provider =
                resolve_provider(agent_args.provider.as_deref(), agent_args.root.as_deref())?;
            debug!("Resolved provider: {}", provider);
            let display_name = capitalize(&provider);
            run_agent_action(AgentActionParams {
                agent_name: display_name,
                provider,
                provider_explicit: agent_args.provider.is_some(),
                action,
                system_prompt: agent_args.system_prompt,
                model: agent_args.model,
                root: agent_args.root,
                auto_approve: agent_args.auto_approve,
                add_dirs: agent_args.add_dirs,
                show_usage: agent_args.show_usage,
                quiet,
                verbose,
                worktree: session_isolation.worktree,
                sandbox: session_isolation.sandbox,
                size: agent_args.size,
                json_mode,
                json_schema,
                json_stream,
                session: session_isolation.session,
                max_turns: agent_args.max_turns,
                exit_on_failure,
                context_session,
                session_metadata: {
                    let meta = metadata_args.unwrap_or_default();
                    crate::session_setup::SessionMetadata {
                        name: meta.name,
                        description: meta.description,
                        tags: meta.tags,
                    }
                },
            })
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
