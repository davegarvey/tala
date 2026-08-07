use std::collections::HashMap;
use std::io::Read;
use std::process;
use std::time::Duration;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use serde_json::json;

use crate::models::*;
use crate::store;

fn fail(json: bool, msg: impl std::fmt::Display, code: &str) -> ! {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({"error": format!("{}", msg), "code": code})
        );
    } else {
        eprintln!("Error: {}", msg);
    }
    process::exit(1);
}

/// Exit code for benign blocking timeouts (wait/send --wait running out of
/// time with no new messages). Deliberately distinct from clap's usage-error
/// code 2: a timeout is not a usage error, and scripts must be able to tell
/// "nothing happened yet" apart from "you called me wrong".
const EXIT_TIMEOUT: i32 = 3;

#[derive(Parser)]
#[command(
    name = "tala",
    about = "Agent-to-agent messaging for AI coding tools",
    long_about = "tala is a lightweight messaging tool for AI agents working across projects.\n\nSend messages with `tala send`, wait for replies with `tala wait`, stream a session with `tala stream`,\nor listen to all sessions with `tala listen`.\n\nUse `tala wait --new-session` to wait for a session with an incoming message from another agent.\n\nEvery command supports --json for structured output.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize tala config for this project directory (sets agent name used when sending messages)
    Init {
        #[arg(help = "Agent name for this project (defaults to directory name)")]
        name: Option<String>,
    },

    /// Set or show the active session for this project directory
    #[command(
        after_help = "See also: tala session (show, rename, reopen) for advanced session management"
    )]
    Use {
        #[arg(help = "Session ID to set as active (omit to show current)")]
        session_id: Option<String>,
        #[arg(long, help = "Clear the active session")]
        clear: bool,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Send a message to a session. Use `tala session create` to create a session without a message.
    #[command(
        after_help = "Use --wait / -w to block until a reply arrives.\nUse `tala session create --name` to create a named session.\nUse --stdin or pipe content for messages with special characters (backticks, quotes, leading dashes).\nUse `--` to separate options from message content, e.g. `tala send -- --my-flags`.\n\nEXIT CODES: 0 = sent (or reply received with --wait); 3 = --wait timed out; 1 = error"
    )]
    Send {
        #[arg(help = "Session ID (positional, or use --session/-s)")]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(help = "Message content (omit to read from piped stdin)")]
        message: Option<String>,
        #[arg(
            long = "message-file",
            help = "Read message content from a file (use - for filename to use piped stdin)"
        )]
        message_file: Option<String>,
        #[arg(
            long,
            help = "Read message content from stdin (bypasses shell interpretation)"
        )]
        stdin: bool,
        #[arg(
            long,
            short = 'w',
            help = "Wait for a reply after sending (default: return immediately)"
        )]
        wait: bool,
        #[arg(long = "sender", help = "Override the sender name")]
        sender_name: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, short = 'q', help = "Suppress confirmation output")]
        quiet: bool,
        #[arg(long, help = "Seconds to wait for a reply (default: 60)")]
        timeout: Option<u64>,
    },
    /// Wait for new messages in a session (blocking poll — sends an HTTP request every few seconds).
    /// Use `tala stream` for real-time SSE on a single session, or `tala listen` to observe all sessions.
    /// Use `tala wait --new-session` to wait for a session with an incoming message from another agent.
    #[command(
        after_help = "USAGE:\n  tala wait <session>          Blocking poll — sends periodic HTTP requests\n  tala wait --new-session     Wait for a session with an incoming message from another agent\n\nCOMPARISON:\n  tala stream   Real-time SSE — stays connected, pushes messages immediately (single session)\n  tala listen   Real-time SSE — observe all sessions at once\n  tala check    Non-blocking — show new messages and return immediately\n\nEXIT CODES: 0 = messages received (or new session found); 3 = benign timeout; 2 = usage error; 1 = error\n\nSee also: tala history (transcript), tala session (manage sessions)"
    )]
    Wait {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, help = "Seconds to wait before timing out (default: 60)")]
        timeout: Option<u64>,
        #[arg(long, help = "Only return messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Maximum number of messages to return (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, help = "Only return messages from this sender")]
        from: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(
            long = "new-session",
            help = "Wait for a new session to be created (ignores other args)"
        )]
        r#new: bool,
    },
    /// Stream new messages as they arrive for a single session (real-time SSE — stays connected and pushes messages).
    /// Use `tala wait` for a blocking poll (request/response), or `tala listen` to observe all sessions.
    #[command(
        name = "stream",
        after_help = "USAGE:\n  tala stream <session>   Real-time SSE — stays connected, pushes messages immediately (single session)\n\nCOMPARISON:\n  tala wait     Blocking poll — sends periodic HTTP requests, good for scripts and CI\n  tala listen   Real-time SSE — observe all sessions at once\n  tala check    Non-blocking — show new messages and return immediately\n\nSee also: tala history (transcript)"
    )]
    Stream {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, help = "Only stream messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Maximum number of messages to stream (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, help = "Seconds to stay connected before disconnecting")]
        timeout: Option<u64>,
    },

    /// View conversation transcript
    #[command(
        after_help = "See also: tala wait (blocking poll), tala listen (all sessions), tala stream (real-time SSE)"
    )]
    History {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, help = "Only show messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Only show messages from this sender")]
        from: Option<String>,
        #[arg(long, help = "Maximum number of messages to show (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Observe all sessions for new messages (real-time SSE across all sessions).
    /// Use `tala stream` for a single session, or `tala wait` for a blocking poll.
    #[command(
        after_help = "USAGE:\n  tala listen                Real-time SSE — observe all sessions at once\n  tala listen --since <n>   Skip history replay (only messages with ID > n)\n  tala listen --from <name> Filter messages from a specific sender\n  tala listen --match <text> Filter messages containing text\n  tala listen --name <name> Filter by session name\n\nCOMPARISON:\n  tala stream   Real-time SSE — single session\n  tala wait     Blocking poll — sends periodic HTTP requests, good for scripts and CI\n  tala check    Non-blocking -- show new messages and return immediately\n\nSee also: tala history (transcript)"
    )]
    Listen {
        #[arg(long, help = "Only show messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Only show messages containing this text")]
        r#match: Option<String>,
        #[arg(long, help = "Only show messages from this sender")]
        from: Option<String>,
        #[arg(long, help = "Only show messages in sessions with matching name")]
        name: Option<String>,
        #[arg(
            long,
            help = "Seconds to stay connected before disconnecting (default: 60, 0 = no timeout)"
        )]
        timeout: Option<u64>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },

    /// Show new messages since last check (non-blocking)
    #[command(
        after_help = "See also: tala wait (blocking poll), tala listen (all sessions), tala stream (real-time SSE), tala history (transcript)"
    )]
    Check {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// List all sessions
    List {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Discover agents in other projects (scans parent directories for tala projects)
    #[command(
        after_help = "Scans up to 3 parent directories and their siblings for .tala/config.json files"
    )]
    Discover {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// List all active agents (unique senders across open sessions)
    #[command(after_help = "See also: tala discover (cross-project agent discovery)")]
    Agents {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Close a session
    Close {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, short = 'q', help = "Suppress confirmation output")]
        quiet: bool,
    },
    /// Show daemon status
    Status {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Stop the background daemon process
    Stop,
    #[command(hide = true)]
    Daemon,
    /// Manage sessions
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
}

#[derive(Subcommand)]
pub enum SessionCommands {
    /// List all sessions
    #[command(after_help = "Alias: tala list")]
    List {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Close a session by ID
    #[command(after_help = "Alias: tala close")]
    Close {
        #[arg(help = "Session ID to close")]
        session_id: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Show session details
    Show {
        #[arg(help = "Session ID to show")]
        session_id: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Rename a session
    Rename {
        #[arg(help = "Session ID to rename")]
        session_id: String,
        #[arg(help = "New name for the session")]
        name: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, help = "Force rename even if session already has a name")]
        force: bool,
    },
    /// Reopen a closed session
    Reopen {
        #[arg(help = "Session ID to reopen")]
        session_id: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Create a new empty session
    Create {
        #[arg(
            long,
            short = 'n',
            help = "Session name (shown in list and check output)"
        )]
        name: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { name } => cmd_init(name).await,
        Commands::Use {
            session_id,
            clear,
            json,
        } => cmd_use(session_id, clear, json).await,
        Commands::Send {
            session,
            session_arg,
            message,
            message_file,
            stdin,
            wait,
            sender_name,
            json,
            quiet,
            timeout,
        } => {
            // A non-sess_ positional alongside a message means the user typo'd the
            // target (or forgot -s/--session): error instead of silently sending
            // to the active session (B026 family).
            if let (Some(target), Some(_)) = (&session, &message) {
                if !target.starts_with("sess_") {
                    anyhow::bail!(
                        "Unknown session '{}'. Session IDs look like 'sess_...' (use `tala list` to see them, or `-s/--session` for the flag form).",
                        target
                    );
                }
            }
            let session_flag = session_arg.is_some();
            let resolved_session = session_arg
                .or_else(|| session.as_ref().filter(|s| s.starts_with("sess_")).cloned());
            let resolved_message = message.or_else(|| {
                if session_flag {
                    session
                } else {
                    session.filter(|s| !s.starts_with("sess_"))
                }
            });
            if stdin && resolved_message.is_some() {
                eprintln!("Warning: --stdin is set, ignoring positional message argument");
            }
            cmd_send(
                resolved_session,
                resolved_message,
                message_file,
                stdin,
                wait,
                sender_name.as_deref(),
                json,
                quiet,
                timeout,
            )
            .await
        }
        Commands::Wait {
            session,
            session_arg,
            timeout,
            since,
            limit,
            from,
            json,
            r#new,
        } => {
            if r#new {
                cmd_wait_new(timeout, json).await
            } else {
                cmd_wait(session.or(session_arg), timeout, since, limit, from, json).await
            }
        }
        Commands::Stream {
            session,
            session_arg,
            since,
            limit,
            json,
            timeout,
        } => cmd_watch(session.or(session_arg), since, limit, json, timeout).await,
        Commands::History {
            session,
            session_arg,
            since,
            from,
            limit,
            json,
        } => cmd_recap(session.or(session_arg), since, from, limit, json).await,
        Commands::Listen {
            since,
            r#match,
            from,
            name,
            timeout,
            json,
        } => cmd_listen(since, r#match, from, name, timeout, json).await,
        Commands::List { json } => cmd_list(json).await,
        Commands::Discover { json } => cmd_discover(json).await,
        Commands::Agents { json } => cmd_agents(json).await,
        Commands::Close {
            session,
            session_arg,
            json,
            quiet,
        } => cmd_close(session.or(session_arg), json, quiet).await,
        Commands::Check { json } => cmd_whatsup(json).await,
        Commands::Status { json } => cmd_status(json).await,
        Commands::Stop => cmd_stop().await,
        Commands::Daemon => crate::daemon::run_daemon().await,
        Commands::Session { command } => match command {
            SessionCommands::List { json } => cmd_list(json).await,
            SessionCommands::Close { session_id, json } => {
                cmd_close(Some(session_id), json, false).await
            }
            SessionCommands::Show { session_id, json } => cmd_session_show(session_id, json).await,
            SessionCommands::Rename {
                session_id,
                name,
                json,
                force,
            } => cmd_session_rename(session_id, name, json, force).await,
            SessionCommands::Reopen { session_id, json } => {
                cmd_session_reopen(session_id, json).await
            }
            SessionCommands::Create { name, json } => cmd_session_create(name, json).await,
        },
    }
}

fn daemon_home_display() -> String {
    let path = store::tala_home();
    if let Ok(th) = std::env::var("TALA_HOME") {
        format!("{} (from TALA_HOME={})", path.display(), th)
    } else {
        path.display().to_string()
    }
}

async fn ensure_daemon_running() -> anyhow::Result<(String, u16)> {
    // Check if daemon.json exists and daemon is reachable
    if let Ok(info) = store::read_daemon_json().await {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_default();
        let alive = client
            .get(format!("http://{}:{}/api/status", info.host, info.port))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if alive {
            return Ok((info.host, info.port));
        }
        // Stale daemon.json — clean up and restart
        store::remove_daemon_json().await;
    }

    // Start daemon
    let home = daemon_home_display();
    std::process::Command::new(std::env::current_exe()?)
        .arg("daemon")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .context("failed to start daemon")?;

    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Ok(info) = store::read_daemon_json().await {
            return Ok((info.host, info.port));
        }
    }

    let daemon_path = store::tala_home().join("daemon.json");
    if !daemon_path.exists() {
        bail!(
            "Daemon not found at {}/daemon.json. Check TALA_HOME is set correctly.",
            home
        );
    } else {
        bail!("daemon failed to start within 5 seconds (daemon.json exists at {}/daemon.json but daemon is not reachable)", home);
    }
}

fn daemon_url(host: &str, port: u16, path: &str) -> String {
    format!("http://{}:{}{}", host, port, path)
}

/// Build the `&seen=` query suffix for wait-new requests: the waiter's
/// per-session read cursors (URL-encoded JSON map) so the daemon can exclude
/// sessions the waiter already knows (B029). Empty when no cursors exist.
async fn wait_new_seen_param() -> String {
    let cursors = store::read_cursors().await;
    if cursors.is_empty() {
        return String::new();
    }
    match serde_json::to_string(&cursors) {
        Ok(json) => format!("&seen={}", percent_encode(&json)),
        Err(_) => String::new(),
    }
}

async fn resolve_session_id(
    host: &str,
    port: u16,
    session_arg: Option<&str>,
    cmd_name: &str,
) -> anyhow::Result<String> {
    if let Some(id) = session_arg {
        return Ok(id.to_string());
    }

    if let Some(id) = store::read_active_session().await {
        return Ok(id);
    }

    let url = daemon_url(host, port, "/api/sessions");
    let resp = reqwest::get(&url).await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;
    let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

    match active.len() {
        0 => bail!("No active sessions. Start one with `tala send`"),
        1 => Ok(active[0].id.clone()),
        _ => {
            let ids: Vec<&str> = active.iter().map(|s| s.id.as_str()).collect();
            bail!(
                "Multiple open sessions: {}. Specify one with `tala {} <session>` or set one with `tala use <session>`",
                ids.join(", "),
                cmd_name
            );
        }
    }
}

async fn cmd_init(name: Option<String>) -> anyhow::Result<()> {
    let tala_dir = std::path::PathBuf::from(".tala");
    tokio::fs::create_dir_all(&tala_dir).await?;

    let config_path = tala_dir.join("config.json");
    if config_path.exists() {
        eprintln!("./.tala/config.json already exists");
    } else {
        let project_name = name.unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "project".to_string())
        });
        let config = json!({ "name": project_name });
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&config)?).await?;
        println!("Created ./.tala/config.json with name: {}", project_name);
    }

    install_opencode_skills().await?;
    Ok(())
}

async fn install_opencode_skills() -> anyhow::Result<()> {
    let opencode_dir = std::path::PathBuf::from(".opencode");
    if !opencode_dir.exists() {
        return Ok(());
    }

    let skill_dir = opencode_dir.join("skills").join("tala");
    tokio::fs::create_dir_all(&skill_dir).await?;

    let skill_path = skill_dir.join("SKILL.md");
    // Canonical docs live in the repo (.opencode/...) and are embedded at build time
    // so the binary always ships exactly the docs that are committed.
    let skill = include_str!("../.opencode/skills/tala/SKILL.md");
    tokio::fs::write(&skill_path, skill).await?;
    println!("Created .opencode/skills/tala/SKILL.md");

    let commands_dir = opencode_dir.join("commands");
    tokio::fs::create_dir_all(&commands_dir).await?;
    let command_path = commands_dir.join("tala.md");
    let command = include_str!("../.opencode/commands/tala.md");
    tokio::fs::write(&command_path, command).await?;
    println!("Created .opencode/commands/tala.md");
    Ok(())
}

async fn cmd_use(session_id: Option<String>, clear: bool, json_output: bool) -> anyhow::Result<()> {
    if clear {
        store::clear_active_session().await?;
        if json_output {
            println!("{}", serde_json::json!({"status": "cleared"}));
        } else {
            println!("Active session cleared");
        }
        return Ok(());
    }

    if let Some(input) = session_id {
        let (host, port) = ensure_daemon_running().await?;

        // Try name match first (more meaningful to users)
        let url = daemon_url(&host, port, "/api/sessions");
        let resp = reqwest::get(&url).await?;
        let sessions: Vec<SessionSummary> = resp.json().await?;
        let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

        let name_matches: Vec<&SessionSummary> = active
            .iter()
            .filter(|s| s.name.as_deref() == Some(&input))
            .copied()
            .collect();

        if name_matches.len() == 1 {
            let id = &name_matches[0].id;
            store::write_active_session(id).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({"session_id": id, "name": name_matches[0].name, "message_count": name_matches[0].message_count, "status": "active"})
                );
            } else {
                let name = name_matches[0].name.as_deref().unwrap_or("-");
                println!(
                    "Active session: {}  ({})  {} msgs",
                    id, name, name_matches[0].message_count
                );
            }
            return Ok(());
        } else if name_matches.len() > 1 {
            bail!(
                "Multiple sessions named '{}'. Use session ID instead.",
                input
            );
        }

        // Fall back to ID match (exact or prefix)
        let id_matches: Vec<&SessionSummary> = active
            .iter()
            .filter(|s| s.id == input || s.id.starts_with(&input))
            .copied()
            .collect();

        if id_matches.len() == 1 {
            let id = &id_matches[0].id;
            store::write_active_session(id).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({"session_id": id, "name": id_matches[0].name, "message_count": id_matches[0].message_count, "status": "active"})
                );
            } else {
                let name = id_matches[0].name.as_deref().unwrap_or("-");
                println!(
                    "Active session: {}  ({})  {} msgs",
                    id, name, id_matches[0].message_count
                );
            }
            return Ok(());
        } else if id_matches.len() > 1 {
            let ids: Vec<&str> = id_matches.iter().map(|s| s.id.as_str()).collect();
            bail!(
                "Multiple sessions match '{}': {}. Use a more specific ID.",
                input,
                ids.join(", ")
            );
        }

        // Check if input matches a closed session
        let closed_match: Vec<&SessionSummary> = sessions
            .iter()
            .filter(|s| s.closed && (s.id == input || s.id.starts_with(&input)))
            .collect();
        if !closed_match.is_empty() {
            bail!(
                "Session '{}' is closed. Use `tala session reopen {}` to open it, then `tala use {}` to make it active",
                closed_match[0].id,
                closed_match[0].id,
                closed_match[0].id
            );
        }

        bail!("No active session named or matching '{}'", input);
    }

    match store::read_active_session().await {
        Some(id) => {
            let (host, port) = ensure_daemon_running().await?;
            let url = daemon_url(&host, port, &format!("/api/sessions/{}", id));
            let resp = reqwest::Client::new().get(&url).send().await;
            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(session) = r.json::<SessionSummary>().await {
                        if json_output {
                            println!(
                                "{}",
                                serde_json::json!({"session_id": id, "name": session.name, "message_count": session.message_count})
                            );
                        } else {
                            let name = session.name.as_deref().unwrap_or("-");
                            println!(
                                "Active session: {}  ({})  {} msgs",
                                id, name, session.message_count
                            );
                        }
                        return Ok(());
                    }
                }
                _ => {}
            }
            // Fallback if API call fails
            if json_output {
                println!("{}", serde_json::json!({"session_id": id}));
            } else {
                println!("Active session: {}", id);
            }
        }
        None => {
            if json_output {
                println!("{}", serde_json::json!({"session_id": null}));
            } else {
                let (host, port) = ensure_daemon_running().await?;
                let url = daemon_url(&host, port, "/api/sessions");
                if let Ok(resp) = reqwest::get(&url).await {
                    if let Ok(sessions) = resp.json::<Vec<SessionSummary>>().await {
                        let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
                        if active.is_empty() {
                            println!("No open sessions. Start one with `tala send`.");
                        } else {
                            println!("Available sessions:\n");
                            for s in &active {
                                let name = s.name.as_deref().unwrap_or("-");
                                println!("  {}  {}  {} msgs", s.id, name, s.message_count);
                            }
                            println!("\nSet one with `tala use <session-id>`.");
                        }
                        return Ok(());
                    }
                }
                println!("No active session set. Use `tala use <session-id>` to set one.");
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn auto_create_session(
    host: &str,
    port: u16,
    sender_override: Option<&str>,
    quiet: bool,
    json_output: bool,
    session_name: Option<String>,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let url = daemon_url(host, port, "/api/sessions");
    let sender = store::get_sender_name(sender_override);
    let resp = client
        .post(&url)
        .json(&CreateSessionRequest {
            message: None,
            sender: Some(sender),
            name: session_name,
        })
        .send()
        .await?;
    let session: CreateSessionResponse = resp.json().await?;
    store::write_active_session(&session.id).await?;
    // B029: a session the waiter itself creates is "seen" from birth (cursor
    // entry) — never a candidate for its own `wait --new-session` scan.
    let _ = store::write_cursor(&session.id, 0).await;
    if !quiet && !json_output {
        println!("{}", session.id);
    }
    Ok(session.id)
}

async fn try_read_piped_stdin() -> Option<String> {
    tokio::time::timeout(Duration::from_millis(500), async {
        tokio::task::spawn_blocking(|| {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok()?;
            let trimmed = buf.trim_end_matches('\n').to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .await
        .ok()?
    })
    .await
    .ok()?
}

#[allow(clippy::too_many_arguments)]
async fn cmd_send(
    session_arg: Option<String>,
    message: Option<String>,
    message_file: Option<String>,
    stdin_flag: bool,
    should_wait: bool,
    sender_override: Option<&str>,
    json_output: bool,
    quiet: bool,
    chat_timeout: Option<u64>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let has_content = message.is_some() || message_file.is_some() || stdin_flag;

    if !has_content && session_arg.is_none() && store::read_active_session().await.is_none() {
        let mut hint = String::new();
        if let Ok(req) = reqwest::get(&daemon_url(&host, port, "/api/sessions")).await {
            if let Ok(sessions) = req.json::<Vec<SessionSummary>>().await {
                let open: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
                if !open.is_empty() {
                    hint = format!("\nOpen session: {} {}. Set it with `tala use` or target it with `tala send --session <id>`.",
                        open[0].id,
                        open[0].name.as_deref().unwrap_or(""),
                    );
                }
            }
        }
        fail(
            json_output,
            format!(
                "Nothing to send. Use `tala session create` to create a session without a message.{}",
                hint
            ),
            "NOTHING_TO_SEND",
        );
    }

    // Note: unknown/typo'd flags are now rejected by clap (no allow_hyphen_values
    // on send positionals), so a positional message starting with '--' can only
    // arrive via the explicit `--` separator — which is the documented correct
    // usage and needs no warning (B015).
    // Resolve content
    let content = if let Some(f) = message_file {
        if f == "-" {
            let piped = try_read_piped_stdin().await
                .ok_or_else(|| anyhow::anyhow!("No piped input. Use `--stdin` for explicit stdin, or provide a filename for --message-file"))?;
            piped
        } else {
            tokio::fs::read_to_string(&f)
                .await?
                .trim_end_matches('\n')
                .to_string()
        }
    } else if stdin_flag {
        try_read_piped_stdin().await.ok_or_else(|| {
            anyhow::anyhow!("No message provided via stdin (use `--stdin` flag with piped input)")
        })?
    } else if let Some(msg) = &message {
        if msg.is_empty() {
            fail(json_output, "Message cannot be empty.", "EMPTY_MESSAGE");
        }
        msg.clone()
    } else {
        try_read_piped_stdin().await
            .ok_or_else(|| anyhow::anyhow!("No message provided. Use a positional argument, --message-file <path>, --stdin, or pipe to stdin"))?
    };

    // Resolve session: explicit, active, stale-replace, or auto-create
    let session_id = if let Some(id) = session_arg.clone() {
        id
    } else if let Some(id) = store::read_active_session().await {
        // Validate active session still exists and is open
        let check_url = daemon_url(&host, port, &format!("/api/sessions/{}", id));
        let check = reqwest::Client::new().get(&check_url).send().await;
        match check {
            Ok(r) if r.status().is_success() => {
                let session: Session = r.json().await?;
                if session.closed {
                    store::clear_active_session().await?;
                    let msg = format!(
                        "Session {} is closed. Use `tala session reopen {}` to reopen it.",
                        id, id
                    );
                    fail(json_output, &msg, "SESSION_CLOSED");
                }
                id
            }
            _ => {
                // Stale active session — replace with a new one
                store::clear_active_session().await?;
                auto_create_session(&host, port, sender_override, quiet, json_output, None).await?
            }
        }
    } else {
        // No active session — check if any sessions exist
        let client = reqwest::Client::new();
        let url = daemon_url(&host, port, "/api/sessions");
        let resp = client.get(&url).send().await?;
        let sessions: Vec<SessionSummary> = resp.json().await?;
        let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
        match active.len() {
            0 => {
                auto_create_session(&host, port, sender_override, quiet, json_output, None).await?
            }
            1 => {
                if !quiet && !json_output {
                    let name = active[0].name.as_deref().unwrap_or("-");
                    eprintln!(
                        "Sending to session {} ({})  {} msgs",
                        active[0].id, name, active[0].message_count
                    );
                }
                active[0].id.clone()
            }
            _ => {
                let mut msg = "No active session set.".to_string();
                for s in &active {
                    let name = s.name.as_deref().unwrap_or("-");
                    msg.push_str(&format!("\n  {}  {}", s.id, name));
                }
                msg.push_str("\nSet one with `tala use <id>`");
                fail(json_output, &msg, "NO_ACTIVE_SESSION");
            }
        }
    };

    send_content(
        session_id,
        &content,
        sender_override,
        should_wait,
        chat_timeout,
        json_output,
        quiet,
        &host,
        port,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn send_content(
    session_id: String,
    content: &str,
    sender_override: Option<&str>,
    should_wait: bool,
    chat_timeout: Option<u64>,
    json_output: bool,
    quiet: bool,
    host: &str,
    port: u16,
) -> anyhow::Result<()> {
    let sender = store::get_sender_name(sender_override);
    let client = reqwest::Client::new();
    let url = daemon_url(
        host,
        port,
        &format!("/api/sessions/{}/messages", session_id),
    );

    let req = SendMessageRequest {
        sender,
        content: content.to_string(),
    };
    let resp = client.post(&url).json(&req).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        let (msg, code) = if err.error.contains("closed") {
            (
                format!(
                    "Session {} is closed. Use `tala session reopen {}` to reopen it.",
                    session_id, session_id
                ),
                "SESSION_CLOSED",
            )
        } else {
            (err.error, "SESSION_NOT_FOUND")
        };
        fail(json_output, &msg, code);
    }

    let msg: SendMessageResponse = resp.json().await?;
    let _ = store::write_cursor(&msg.session_id, msg.id).await;

    if !should_wait {
        if json_output {
            println!("{}", serde_json::to_string(&msg).unwrap());
        } else if !quiet {
            println!("✓ Sent message {} to session {}", msg.id, msg.session_id);
        }
        return Ok(());
    }

    let spinner = if !json_output && !quiet {
        eprint!("⏎ Waiting for reply");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let spinner = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                eprint!(".");
                let _ = std::io::Write::flush(&mut std::io::stderr());
            }
        });
        Some(spinner)
    } else {
        if !json_output && !quiet {
            eprint!("⏎ Waiting for reply...");
            let _ = std::io::Write::flush(&mut std::io::stderr());
        }
        None
    };
    let mut wait_url = format!("/api/sessions/{}/wait?since={}", session_id, msg.id);
    if let Some(to) = chat_timeout {
        wait_url = format!("{}&timeout_secs={}", wait_url, to);
    }
    let wait_url = daemon_url(host, port, &wait_url);
    let wait_resp = client.get(&wait_url).send().await?;
    if let Some(s) = spinner {
        s.abort();
        let _ = s.await;
    }
    let result: WaitResponse = wait_resp.json().await?;

    if !json_output && !quiet {
        eprintln!();
    }
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
        if result.timeout {
            process::exit(EXIT_TIMEOUT);
        }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!(
            "[timeout after {}s, no reply]",
            result.timeout_after.unwrap_or(0)
        );
        process::exit(EXIT_TIMEOUT);
    } else {
        for m in &result.messages {
            println!("{}: {}", m.sender, m.content);
        }
    }
    Ok(())
}

async fn cmd_wait(
    session_arg: Option<String>,
    timeout_secs: Option<u64>,
    since: Option<u64>,
    limit: Option<usize>,
    from: Option<String>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let client = reqwest::Client::new();

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(60);
    let wait_timeout = timeout_secs.unwrap_or(default_timeout);

    // B021: identify the reader so the daemon records read receipts.
    let sender_param = format!(
        "&sender={}",
        store::read_project_config()
            .await
            .or_else(|| Some(store::get_default_sender()))
            .unwrap_or_else(|| "unknown".to_string())
    );

    loop {
        let sid = if let Some(id) = session_arg.clone() {
            if !json_output {
                eprintln!(
                    "Waiting for messages in session {} (timeout: {}s)...",
                    id, wait_timeout
                );
            }
            id
        } else if let Some(id) = store::read_active_session().await {
            if !json_output {
                eprintln!(
                    "Waiting for messages in session {} (timeout: {}s)...",
                    id, wait_timeout
                );
            }
            id
        } else {
            let url = daemon_url(&host, port, "/api/sessions");
            let resp = client.get(&url).send().await?;
            let sessions: Vec<SessionSummary> = resp.json().await?;
            let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

            match active.len() {
                0 => {
                    if !json_output {
                        eprintln!(
                            "No active sessions. Waiting for a new session (timeout: {}s)...",
                            wait_timeout
                        );
                    }
                    let sender = store::read_project_config()
                        .await
                        .or_else(|| Some(store::get_default_sender()));
                    let sender_param = match sender {
                        Some(s) => format!("&sender={}", s),
                        None => String::new(),
                    };
                    let seen_param = wait_new_seen_param().await;
                    let new_url = daemon_url(
                        &host,
                        port,
                        &format!(
                            "/api/sessions/wait-new?timeout_secs={}{}{}",
                            wait_timeout, sender_param, seen_param
                        ),
                    );
                    let resp = client.get(&new_url).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    if json_output {
                        println!("{}", serde_json::to_string(&result).unwrap());
                        if result.get("timeout") == Some(&serde_json::json!(true)) {
                            process::exit(EXIT_TIMEOUT);
                        }
                        return Ok(());
                    }
                    let sid_val = result
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            if result.get("timeout") == Some(&serde_json::json!(true)) {
                                anyhow::anyhow!(
                                    "timed out after {}s, no new session",
                                    result["timeout_after"].as_u64().unwrap_or(wait_timeout)
                                )
                            } else {
                                anyhow::anyhow!("failed to wait for new session")
                            }
                        })?
                        .to_string();
                    store::write_active_session(&sid_val).await?;
                    eprintln!("New session: {}", sid_val);
                    sid_val
                }
                1 => {
                    let sid_val = active[0].id.clone();
                    if !json_output {
                        eprintln!(
                            "Waiting for new messages in session {} (timeout: {}s)...",
                            sid_val, wait_timeout
                        );
                    }
                    sid_val
                }
                _ => {
                    if json_output {
                        let sessions_json: Vec<serde_json::Value> = active
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "id": s.id,
                                    "name": s.name,
                                    "message_count": s.message_count
                                })
                            })
                            .collect();
                        println!(
                            "{}",
                            serde_json::json!({
                                "sessions": sessions_json,
                                "error": "Use 'tala use <id>' to select a session"
                            })
                        );
                    } else {
                        println!("Multiple open sessions. Use `tala use <id>` to select one:\n");
                        for s in &active {
                            let name = s.name.as_deref().unwrap_or("-");
                            println!("  {}  {}  {} msgs", s.id, name, s.message_count);
                        }
                    }
                    process::exit(0);
                }
            }
        };

        let since_id = if let Some(s) = since {
            s
        } else {
            let msgs_url = daemon_url(
                &host,
                port,
                &format!("/api/sessions/{}/messages?since=0", sid),
            );
            match client.get(&msgs_url).send().await {
                Ok(resp) => {
                    let msgs: Vec<Message> = resp.json().await.unwrap_or_default();
                    msgs.iter().map(|m| m.id).max().unwrap_or(0)
                }
                Err(_) => 0,
            }
        };

        let mut path = format!(
            "/api/sessions/{}/wait?since={}&timeout_secs={}{}",
            sid, since_id, wait_timeout, sender_param
        );
        if let Some(l) = limit.filter(|&l| l > 0) {
            path = format!("{}&limit={}", path, l);
        }
        if let Some(ref f) = from {
            path = format!("{}&from={}", path, f);
        }

        let url = daemon_url(&host, port, &path);

        let spinner = if !json_output {
            let spinner = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    eprint!(".");
                    let _ = std::io::Write::flush(&mut std::io::stderr());
                }
            });
            Some(spinner)
        } else {
            None
        };

        let resp = client.get(&url).send().await?;

        if let Some(s) = spinner {
            s.abort();
            let _ = s.await;
        }

        if !resp.status().is_success() {
            let err: ErrorResponse = resp.json().await?;
            if session_arg.is_none() && err.error.to_lowercase().contains("session not found") {
                store::clear_active_session().await?;
                if !json_output {
                    eprintln!("Active session was stale. Re-discovering...");
                }
                continue;
            }
            fail(json_output, &err.error, "SESSION_NOT_FOUND");
        }

        let result: WaitResponse = resp.json().await?;

        if json_output {
            println!("{}", serde_json::to_string(&result).unwrap());
            if result.timeout {
                process::exit(EXIT_TIMEOUT);
            }
        } else if result.closed {
            println!("[session closed]");
        } else if result.timeout {
            println!(
                "timeout after {}s, no new messages",
                result.timeout_after.unwrap_or(0)
            );
            process::exit(EXIT_TIMEOUT);
        } else {
            let _ = store::write_active_session(&sid).await;
            for msg in &result.messages {
                println!(
                    "[sess {}] [{}] {} ({}):\n    {}",
                    sid,
                    msg.id,
                    msg.sender,
                    msg.timestamp.format("%H:%M:%S"),
                    msg.content
                );
            }
        }

        break;
    }
    Ok(())
}

async fn cmd_watch(
    session_arg: Option<String>,
    since: Option<u64>,
    limit: Option<usize>,
    json_output: bool,
    timeout: Option<u64>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "stream").await?;

    let since_id = since.unwrap_or(0);
    let mut path = format!("/api/sessions/{}/events?since={}", session_id, since_id);
    if let Some(l) = limit.filter(|&l| l > 0) {
        path = format!("{}&limit={}", path, l);
    }
    let url = daemon_url(&host, port, &path);

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    // B007: visible connection status (text → stdout, --json → stderr).
    if json_output {
        eprintln!(
            "[stream] connected to tala daemon at {}:{} (session {}, since id {})",
            host, port, session_id, since_id
        );
    } else {
        println!(
            "Streaming session {} from tala daemon at {}:{} (since id {})...",
            session_id, host, port, since_id
        );
    }

    let timeout_dur = timeout.filter(|&t| t > 0).map(Duration::from_secs);

    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let mut message_count: u64 = 0;

    loop {
        let chunk = if let Some(dur) = timeout_dur {
            match tokio::time::timeout(dur, stream.next()).await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(_) => break,
            }
        } else {
            match stream.next().await {
                Some(chunk) => chunk,
                None => break,
            }
        };
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let event_type = event_block
                .lines()
                .find_map(|line| line.strip_prefix("event: "))
                .unwrap_or("message");
            let mut data = String::new();

            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("data: ") {
                    data = val.to_string();
                }
            }

            match event_type {
                "closed" => {
                    if json_output {
                        println!("{}", json!({"event": "closed"}));
                    } else {
                        println!("[session closed]");
                    }
                    return Ok(());
                }
                "message" => {
                    message_count += 1;
                    if json_output {
                        if let Ok(msg) = serde_json::from_str::<Message>(&data) {
                            let mut obj: serde_json::Value =
                                serde_json::from_str(&data).unwrap_or_default();
                            obj["cursor"] = serde_json::json!(msg.id);
                            println!("{}", serde_json::to_string(&obj).unwrap());
                        } else {
                            println!("{}", data);
                        }
                    } else if let Ok(msg) = serde_json::from_str::<Message>(&data) {
                        println!(
                            "[{}] {} ({}):\n    {}",
                            msg.id,
                            msg.sender,
                            msg.timestamp.format("%H:%M:%S"),
                            msg.content
                        );
                    }
                }
                _ => {}
            }
        }
    }

    if message_count == 0 {
        if json_output {
            println!("[]");
        } else {
            println!("[no messages received]");
        }
    }

    Ok(())
}

async fn cmd_listen(
    since: Option<u64>,
    match_str: Option<String>,
    from: Option<String>,
    name: Option<String>,
    timeout: Option<u64>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let since_id = since.unwrap_or_default();
    let mut path = format!("/api/observe?since={}", since_id);
    if since.is_none() {
        // No explicit --since: replay each session since ITS OWN read cursor.
        // A single global since cannot represent per-session id spaces (B014).
        let cursors = store::read_cursors().await;
        let since_map = serde_json::to_string(&cursors).unwrap_or_else(|_| "{}".to_string());
        path = format!("{}&since_map={}", path, percent_encode(&since_map));
    }
    if let Some(ref m) = match_str {
        path = format!("{}&match={}", path, urlencoding(m));
    }
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }
    if let Some(ref n) = name {
        path = format!("{}&channel={}", path, n);
    }
    // Default timeout to 60s if not specified, unless explicitly set to 0
    let timeout_secs = timeout.filter(|&t| t != 0).or(Some(60u64));
    if let Some(t) = timeout_secs {
        path = format!("{}&timeout_secs={}", path, t);
    }
    let url = daemon_url(&host, port, &path);

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "OBSERVE_ERROR");
    }

    // B007: visible connection status. Text banner goes to stdout (matches the
    // "Waiting for a new session…" convention in wait); in --json mode it goes
    // to stderr so stdout stays a pure JSON event stream.
    if json_output {
        eprintln!(
            "[listen] connected to tala daemon at {}:{} (since id {})",
            host, port, since_id
        );
    } else {
        println!(
            "Listening on tala daemon at {}:{} (all sessions, since id {})...",
            host, port, since_id
        );
    }

    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let mut message_count: u64 = 0;
    let mut max_by_session: HashMap<String, u64> = HashMap::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let mut data = String::new();

            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("data: ") {
                    data = val.to_string();
                }
            }

            let evt = serde_json::from_str::<ObserveEvent>(&data);
            if let Ok(ref evt) = evt {
                if evt.r#type == "message" {
                    message_count += 1;
                }
            }

            if json_output {
                println!("{}", data);
            } else if let Ok(evt) = evt {
                match evt.r#type.as_str() {
                    "message" => {
                        if let Some(msg) = evt.message {
                            let sid = evt.session_id.clone();
                            let entry = max_by_session.entry(sid).or_insert(0);
                            if msg.id > *entry {
                                *entry = msg.id;
                            }
                            let session_label = evt.session_name.unwrap_or(evt.session_id);
                            println!(
                                "[{}] {} ({}):\n    {}",
                                session_label,
                                msg.sender,
                                msg.timestamp.format("%H:%M:%S"),
                                msg.content
                            );
                        }
                    }
                    "closed" => {
                        let session_label = evt.session_name.unwrap_or(evt.session_id);
                        println!("[{}] session closed", session_label);
                    }
                    _ => {}
                }
            }
        }
    }

    // B007: end-of-stream note with a message tally so connected-but-quiet is
    // distinguishable from a dead listener.
    if json_output {
        eprintln!("[listen] connection closed ({} message(s))", message_count);
    } else {
        println!("[connection closed] ({} message(s))", message_count);
    }

    // Advance the per-session read cursor for each session we saw messages in.
    for (sid, mid) in &max_by_session {
        if *mid > store::read_cursor(sid).await {
            let _ = store::write_cursor(sid, *mid).await;
        }
    }

    Ok(())
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
}

async fn cmd_recap(
    session_arg: Option<String>,
    since: Option<u64>,
    from: Option<String>,
    limit: Option<usize>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "recap").await?;

    // B021: identify the reader so the daemon records read receipts.
    let sender_param = format!(
        "&sender={}",
        store::read_project_config()
            .await
            .or_else(|| Some(store::get_default_sender()))
            .unwrap_or_else(|| "unknown".to_string())
    );

    let since_id = since.unwrap_or(0);
    let mut path = format!(
        "/api/sessions/{}/recap?since={}{}",
        session_id, since_id, sender_param
    );
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }
    if let Some(l) = limit.filter(|&l| l > 0) {
        path = format!("{}&limit={}", path, l);
    }

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &path);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let recap: RecapResponse = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&recap).unwrap());
    } else {
        println!(
            "session: {}  |  created: {}  |  closed: {}",
            recap.session.id,
            recap.session.created_at.format("%Y-%m-%d %H:%M:%S"),
            recap.session.closed
        );
        if let Some(c) = recap.cursor {
            println!("cursor: {}", c);
        }
        println!();
        if recap.messages.is_empty() {
            println!("(no messages yet)");
        } else {
            for msg in &recap.messages {
                println!(
                    "[{}] {} ({}):\n    {}\n",
                    msg.id,
                    msg.sender,
                    msg.timestamp.format("%H:%M:%S"),
                    msg.content
                );
            }
        }
    }
    // Mark THIS session read only. Empty sessions have cursor: None — skip the
    // write so an empty history can never reset read state (B025).
    if let Some(c) = recap.cursor {
        store::write_cursor(&session_id, c).await?;
    }
    Ok(())
}

async fn cmd_list(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    let cursors = store::read_cursors().await;
    let active_session = store::read_active_session().await;

    if json_output {
        let mut enriched: Vec<serde_json::Value> = Vec::new();
        for s in &sessions {
            let unread = if s.closed {
                0
            } else {
                let since_id = cursors.get(&s.id).copied().unwrap_or(0);
                compute_session_unread(&host, port, s, since_id).await
            };
            let mut entry = serde_json::to_value(s).unwrap_or_default();
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("unread_count".to_string(), serde_json::json!(unread));
                obj.insert(
                    "active".to_string(),
                    serde_json::json!(active_session.as_deref() == Some(&s.id)),
                );
            }
            enriched.push(entry);
        }
        println!("{}", serde_json::to_string(&enriched).unwrap());
    } else if sessions.is_empty() {
        println!("No sessions");
    } else {
        let name_width = sessions
            .iter()
            .map(|s| s.name.as_deref().unwrap_or("-").len())
            .max()
            .unwrap_or(1)
            .max(4);
        for s in &sessions {
            let status = if s.closed { "closed" } else { "open" };
            let name = s.name.as_deref().unwrap_or("-");
            let marker = if active_session.as_deref() == Some(&s.id) {
                " *"
            } else {
                "  "
            };
            // B021: show readers OTHER than the local identity (self-reads are
            // noise in text; --json exposes the full read_by map).
            let local_identity = store::read_project_config()
                .await
                .or_else(|| Some(store::get_default_sender()));
            let mut readers: Vec<String> = s
                .read_by
                .iter()
                .filter(|(sender, _)| local_identity.as_deref() != Some(sender.as_str()))
                .map(|(sender, id)| format!("{}@{}", sender, id))
                .collect();
            readers.sort();
            let read_suffix = if readers.is_empty() {
                String::new()
            } else {
                format!("  read: {}", readers.join(", "))
            };
            if s.closed {
                println!(
                    "{}  {:width$}  {}  {} msgs{}{}",
                    s.id,
                    name,
                    status,
                    s.message_count,
                    marker,
                    read_suffix,
                    width = name_width
                );
            } else {
                let since_id = cursors.get(&s.id).copied().unwrap_or(0);
                let unread = compute_session_unread(&host, port, s, since_id).await;
                if unread > 0 {
                    println!(
                        "{}  {:width$}  {}  {} msgs ({} new){}{}",
                        s.id,
                        name,
                        status,
                        s.message_count,
                        unread,
                        marker,
                        read_suffix,
                        width = name_width
                    );
                } else {
                    println!(
                        "{}  {:width$}  {}  {} msgs{}{}",
                        s.id,
                        name,
                        status,
                        s.message_count,
                        marker,
                        read_suffix,
                        width = name_width
                    );
                }
            }
        }
    }
    Ok(())
}

async fn compute_session_unread(
    host: &str,
    port: u16,
    session: &SessionSummary,
    cursor: u64,
) -> usize {
    if cursor == 0 && session.message_count == 0 {
        return 0;
    }
    let local_agent = store::read_project_config()
        .await
        .or_else(|| Some(store::get_default_sender()));
    let client = reqwest::Client::new();
    let msgs_url = daemon_url(
        host,
        port,
        &format!("/api/sessions/{}/messages?since={}", session.id, cursor),
    );
    match client.get(&msgs_url).send().await {
        Ok(resp) => {
            let msgs: Vec<Message> = resp.json().await.unwrap_or_default();
            if let Some(ref agent) = local_agent {
                msgs.iter().filter(|m| m.sender != *agent).count()
            } else {
                msgs.len()
            }
        }
        Err(_) => 0,
    }
}

async fn check_tcp_port(host: &str, port: u16) -> bool {
    tokio::net::TcpStream::connect((host, port)).await.is_ok()
}

async fn probe_daemon(host: &str, port: u16, agents: &mut Vec<AgentSummary>) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    let url = format!("http://{}:{}/api/agents", host, port);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            *agents = resp.json::<Vec<AgentSummary>>().await.unwrap_or_default();
            true
        }
        _ => check_tcp_port(host, port).await,
    }
}

async fn try_read_json(path: &std::path::Path) -> Option<serde_json::Value> {
    tokio::fs::read_to_string(path)
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[derive(serde::Serialize)]
struct DiscoveredProject {
    project: String,
    agent_name: String,
    daemon_running: bool,
    agents: Vec<AgentSummary>,
}

async fn cmd_discover(json_output: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let mut discovered: Vec<DiscoveredProject> = Vec::new();

    // Walk up parent directories looking for .tala/config.json
    let mut current = Some(cwd.as_path());
    let mut checked = std::collections::HashSet::new();
    for _ in 0..4 {
        let dir = match current {
            Some(d) => d,
            None => break,
        };
        let tala_config = dir.join(".tala").join("config.json");
        if tala_config.exists() && checked.insert(dir.to_path_buf()) {
            if let Some(config) = try_read_json(&tala_config).await {
                let agent_name = config["name"].as_str().unwrap_or("unknown").to_string();
                let daemon_path = store::tala_home().join("daemon.json");
                let mut daemon_running = false;
                let mut agents: Vec<AgentSummary> = Vec::new();
                if let Some(dinfo) = try_read_json(&daemon_path).await {
                    let host = dinfo["host"].as_str().unwrap_or("127.0.0.1");
                    let port = dinfo["port"].as_u64().unwrap_or(0) as u16;
                    if port > 0 {
                        daemon_running = probe_daemon(host, port, &mut agents).await;
                    }
                }
                discovered.push(DiscoveredProject {
                    project: dir.display().to_string(),
                    agent_name,
                    daemon_running,
                    agents,
                });
            }
        }

        // Check siblings
        if let Some(parent) = dir.parent() {
            if let Ok(mut entries) = tokio::fs::read_dir(parent).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() && path != dir && checked.insert(path.clone()) {
                        let sibling_config = path.join(".tala").join("config.json");
                        if sibling_config.exists() {
                            if let Some(config) = try_read_json(&sibling_config).await {
                                let agent_name =
                                    config["name"].as_str().unwrap_or("unknown").to_string();
                                let daemon_path = store::tala_home().join("daemon.json");
                                let mut daemon_running = false;
                                let mut agents: Vec<AgentSummary> = Vec::new();
                                if let Some(dinfo) = try_read_json(&daemon_path).await {
                                    let host = dinfo["host"].as_str().unwrap_or("127.0.0.1");
                                    let port = dinfo["port"].as_u64().unwrap_or(0) as u16;
                                    if port > 0 {
                                        daemon_running =
                                            probe_daemon(host, port, &mut agents).await;
                                    }
                                }
                                discovered.push(DiscoveredProject {
                                    project: path.display().to_string(),
                                    agent_name,
                                    daemon_running,
                                    agents,
                                });
                            }
                        }
                    }
                }
            }
        }

        current = dir.parent();
    }

    if json_output {
        println!("{}", serde_json::to_string(&discovered).unwrap());
    } else if discovered.is_empty() {
        println!("No other tala projects discovered in parent directories.");
    } else {
        for p in &discovered {
            let daemon_status = if p.daemon_running {
                "running"
            } else {
                "stopped"
            };
            println!(
                "{}  ({})  [daemon: {}]",
                p.project, p.agent_name, daemon_status
            );
            if p.daemon_running && !p.agents.is_empty() {
                for a in &p.agents {
                    println!(
                        "  └ {}  last: {}  {} msgs",
                        a.sender,
                        a.last_seen.format("%Y-%m-%d %H:%M:%S UTC"),
                        a.message_count
                    );
                }
            }
        }
    }

    Ok(())
}

async fn cmd_agents(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/agents");
    let resp = client.get(&url).send().await?;
    let agents: Vec<AgentSummary> = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&agents).unwrap());
    } else if agents.is_empty() {
        println!("No active agents found. Start a session with `tala send`, or try `tala discover` to find agents in other projects.");
    } else {
        for a in &agents {
            println!(
                "{}  last: {}  {} msgs",
                a.sender,
                a.last_seen.format("%Y-%m-%d %H:%M:%S UTC"),
                a.message_count
            );
        }
    }
    Ok(())
}

async fn cmd_close(
    session_arg: Option<String>,
    json_output: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "close").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.delete(&url).send().await?;

    if resp.status().is_success() {
        let result: CloseSessionResponse = resp.json().await?;
        // Clear the active marker whenever the CLOSED session is the active one,
        // regardless of how it was addressed (positional, -s, or `session close`
        // alias which always passes Some(id)). Previously the alias path computed
        // was_active=false and left a dangling * marker + stale active-session
        // file, breaking bare `send` (B028).
        let was_active = store::read_active_session().await.as_deref() == Some(&session_id);
        if was_active {
            store::clear_active_session().await?;
        }
        if json_output {
            let mut out = serde_json::json!({"session_id": session_id, "status": result.status});
            if was_active {
                out["active_cleared"] = serde_json::json!(true);
            }
            println!("{}", out);
        } else if !quiet {
            println!("Session {}: {}", session_id, result.status);
            if was_active {
                eprintln!("Active session was closed and cleared. Use `tala use <session-id>` to set a new one.");
            }
        }
    } else {
        let err: ErrorResponse = resp.json().await?;
        let code = if err.error.contains("closed") {
            "SESSION_CLOSED"
        } else {
            "SESSION_NOT_FOUND"
        };
        fail(json_output, &err.error, code);
    }
    Ok(())
}

async fn cmd_session_show(session_id: String, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let session: Session = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&session).unwrap());
    } else {
        println!("Session: {}", session.id);
        if let Some(ref n) = session.name {
            println!("  Name: {}", n);
        }
        println!(
            "  Created: {}",
            session.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "  Last activity: {}",
            session.last_activity.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "  Status: {}",
            if session.closed { "closed" } else { "open" }
        );
    }
    Ok(())
}

async fn cmd_session_rename(
    session_id: String,
    name: String,
    json_output: bool,
    force: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/rename", session_id));
    let resp = client
        .post(&url)
        .json(&json!({"name": name, "force": force}))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let err: ErrorResponse = resp.json().await?;
        match status.as_u16() {
            409 => fail(json_output, &err.error, "SESSION_ALREADY_NAMED"),
            _ => fail(json_output, &err.error, "SESSION_NOT_FOUND"),
        }
    }

    let result: serde_json::Value = resp.json().await?;
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!(
            "Session {} renamed to '{}'",
            session_id,
            result["name"].as_str().unwrap_or("")
        );
    }
    Ok(())
}

async fn cmd_session_reopen(session_id: String, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/reopen", session_id));
    let resp = client.post(&url).send().await?;

    let status = resp.status();
    if !status.is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let result: serde_json::Value = resp.json().await?;
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!(
            "Session {} reopened (use `tala use {}` to make it active)",
            session_id, session_id
        );
    }
    Ok(())
}

async fn cmd_session_create(session_name: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    auto_create_session(&host, port, None, false, json_output, session_name).await?;
    Ok(())
}

async fn cmd_wait_new(timeout_secs: Option<u64>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let timeout = timeout_secs.unwrap_or(60);
    if !json_output {
        eprintln!("Waiting for a new session (timeout: {}s)...", timeout);
    }
    // B003: identify ourselves so the daemon only delivers sessions with an
    // incoming message from ANOTHER agent (not our own creates).
    let sender = store::read_project_config()
        .await
        .or_else(|| Some(store::get_default_sender()));
    let sender_param = match sender {
        Some(s) => format!("&sender={}", s),
        None => String::new(),
    };
    let seen_param = wait_new_seen_param().await;
    let url = daemon_url(
        &host,
        port,
        &format!(
            "/api/sessions/wait-new?timeout_secs={}{}{}",
            timeout, sender_param, seen_param
        ),
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let result: serde_json::Value = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
        println!("{}", sid);
    } else if result.get("timeout") == Some(&serde_json::json!(true)) {
        eprintln!(
            "timeout after {}s, no new session",
            result["timeout_after"].as_u64().unwrap_or(timeout)
        );
        process::exit(EXIT_TIMEOUT);
    } else if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        fail(json_output, err, "WAIT_NEW_ERROR");
    }
    Ok(())
}

async fn cmd_status(json_output: bool) -> anyhow::Result<()> {
    let info = match store::read_daemon_json().await {
        Ok(info) => info,
        Err(_) => {
            let home = daemon_home_display();
            if json_output {
                println!("{}", serde_json::json!({"running": false, "home": home}));
            } else {
                println!("no daemon running (checked {}/daemon.json)", home);
                println!("Start the daemon by running any tala command, or set TALA_HOME if using a custom location");
            }
            return Ok(());
        }
    };

    let status_url = daemon_url(&info.host, info.port, "/api/status");
    let alive = reqwest::Client::new()
        .get(&status_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if alive {
        let cursors = store::read_cursors().await;
        let total_unread = compute_total_unread(&info.host, info.port, &cursors).await;

        if json_output {
            let resp = serde_json::json!({
                "running": true,
                "pid": info.pid,
                "port": info.port,
                "host": info.host,
                "started_at": info.started_at,
                "total_unread": total_unread,
            });
            println!("{}", serde_json::to_string(&resp).unwrap());
        } else {
            println!("daemon running:");
            println!("  PID:  {}", info.pid);
            println!("  Port: {}", info.port);
            println!("  Host: {}", info.host);
            println!("  Since: {}", info.started_at.format("%Y-%m-%d %H:%M:%S"));
            if total_unread > 0 {
                println!(
                    "  Unread: {} new message(s) across all sessions",
                    total_unread
                );
            } else {
                println!("  Unread: 0 new messages");
            }
        }
    } else {
        let home = daemon_home_display();
        if json_output {
            println!(
                "{}",
                serde_json::json!({"running": false, "stale_daemon_json": true, "home": home})
            );
        } else {
            println!("daemon.json found at {}/daemon.json but daemon is not reachable (may have crashed)", home);
            println!("Try `tala stop` to clean up stale daemon.json, then run your command again.");
        }
    }
    Ok(())
}

async fn compute_total_unread(host: &str, port: u16, cursors: &HashMap<String, u64>) -> usize {
    let local_agent = store::read_project_config().await;
    let client = reqwest::Client::new();
    let url = daemon_url(host, port, "/api/sessions");
    match client.get(&url).send().await {
        Ok(resp) => {
            let sessions: Vec<SessionSummary> = resp.json().await.unwrap_or_default();
            let mut total = 0;
            for s in &sessions {
                let since_id = cursors.get(&s.id).copied().unwrap_or(0);
                let msgs_url = daemon_url(
                    host,
                    port,
                    &format!("/api/sessions/{}/messages?since={}", s.id, since_id),
                );
                if let Ok(resp) = client.get(&msgs_url).send().await {
                    if let Ok(msgs) = resp.json::<Vec<Message>>().await {
                        if let Some(ref agent) = local_agent {
                            total += msgs.iter().filter(|m| m.sender != *agent).count();
                        } else {
                            total += msgs.len();
                        }
                    }
                }
            }
            total
        }
        Err(_) => 0,
    }
}

async fn cmd_whatsup(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let mut cursors = store::read_cursors().await;
    let client = reqwest::Client::new();

    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    let mut all_messages: Vec<Message> = Vec::new();

    // B021: identify the reader so the daemon records read receipts.
    let sender_param = format!(
        "&sender={}",
        store::read_project_config()
            .await
            .or_else(|| Some(store::get_default_sender()))
            .unwrap_or_else(|| "unknown".to_string())
    );

    // Fetch per session since THAT session's own read cursor (B014).
    for s in &sessions {
        let since_id = cursors.get(&s.id).copied().unwrap_or(0);
        let msgs_url = daemon_url(
            &host,
            port,
            &format!(
                "/api/sessions/{}/messages?since={}{}",
                s.id, since_id, sender_param
            ),
        );
        if let Ok(resp) = client.get(&msgs_url).send().await {
            if let Ok(msgs) = resp.json::<Vec<Message>>().await {
                if let Some(max_id) = msgs.iter().map(|m| m.id).max() {
                    cursors.insert(s.id.clone(), max_id);
                }
                all_messages.extend(msgs);
            }
        }
    }

    all_messages.sort_by_key(|m| m.id);

    // "cursor" is kept for backward compatibility: max of the per-session cursors.
    let max_cursor = cursors.values().copied().max().unwrap_or(0);

    if json_output {
        let result = serde_json::json!({
            "cursor": max_cursor,
            "cursors": cursors,
            "messages": all_messages,
        });
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if all_messages.is_empty() {
        println!("No new messages since last check");
    } else {
        // Group messages by session
        let mut by_session: std::collections::BTreeMap<String, Vec<&Message>> =
            std::collections::BTreeMap::new();
        for msg in &all_messages {
            by_session
                .entry(msg.session_id.clone())
                .or_default()
                .push(msg);
        }
        for (sid, msgs) in &by_session {
            // Find session name
            let session_name = sessions
                .iter()
                .find(|s| s.id == *sid)
                .and_then(|s| s.name.clone())
                .unwrap_or_else(|| sid.clone());
            println!("[{}] ({} new message(s))", session_name, msgs.len());
            for msg in msgs {
                println!(
                    "  [{}] {} ({}):\n    {}",
                    msg.id,
                    msg.sender,
                    msg.timestamp.format("%H:%M:%S"),
                    msg.content
                );
            }
            println!();
        }
    }

    // Persist the per-session read cursors.
    for (sid, mid) in &cursors {
        store::write_cursor(sid, *mid).await?;
    }

    if !json_output && !all_messages.is_empty() {
        println!("(read markers updated for {} session(s))", cursors.len());
    }

    Ok(())
}

async fn cmd_stop() -> anyhow::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = store::read_daemon_json().await;
        bail!("stop is not supported on this platform");
    }

    #[cfg(unix)]
    {
        let info = match store::read_daemon_json().await {
            Ok(info) => info,
            Err(_) => {
                println!("daemon is not running");
                return Ok(());
            }
        };
        use libc::kill;
        let pid = info.pid as libc::pid_t;
        // Send SIGTERM directly via the syscall — no external `kill` binary needed.
        // (Containers/slim systems may not ship /bin/kill.)
        let ret = unsafe { kill(pid, libc::SIGTERM) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                // Process already gone — clean up stale daemon.json
                store::remove_daemon_json().await;
                println!("daemon stopped");
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "failed to kill daemon (pid {}): {}",
                pid,
                err
            ));
        }

        for _ in 0..20 {
            if store::read_daemon_json().await.is_err() {
                println!("daemon stopped");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        // Timed out — clean up stale daemon.json and report success
        store::remove_daemon_json().await;
        println!("daemon stopped (stale daemon.json cleaned up)");
        Ok(())
    }
}
