use std::process;
use std::time::Duration;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
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

#[derive(Parser)]
#[command(name = "chit", about = "Agent-to-agent messaging", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize chit in the current project
    Init {
        /// Custom project name
        #[arg(long)]
        name: Option<String>,
    },
    /// Start daemon and create a new session
    Start {
        /// Optional initial message (sends and blocks for reply)
        message: Option<String>,
    },
    /// Send a message to a session (blocks for reply by default)
    #[command(alias = "send")]
    Chat {
        /// Message content (markdown)
        message: String,
        /// Session ID (auto-targets if single session exists)
        #[arg(long, short)]
        session: Option<String>,
        /// Fire-and-forget: don't wait for reply
        #[arg(long, short)]
        ff: bool,
        /// Override sender name
        #[arg(long = "as", name = "sender_name")]
        sender_name: Option<String>,
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
        /// Timeout in seconds when waiting for reply (blocking mode only)
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Wait for the next message in a session
    Wait {
        /// Session ID (positional, for backwards compatibility)
        session: Option<String>,
        /// Session ID (also accepts --session-id for backwards compat)
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
        /// Only return messages with ID greater than this value
        #[arg(long)]
        since: Option<u64>,
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
    },
    /// Show full conversation transcript
    Recap {
        /// Session ID (positional, for backwards compatibility)
        session: Option<String>,
        /// Session ID (also accepts --session-id for backwards compat)
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
    },
    /// List active sessions
    List {
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
    },
    /// Close a session
    Close {
        /// Session ID (positional, for backwards compatibility)
        session: Option<String>,
        /// Session ID (also accepts --session-id for backwards compat)
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
    },
    /// Show daemon status
    Status {
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
    },
    /// Stop the daemon
    Stop,
    /// Run the daemon server (internal)
    #[command(hide = true)]
    Daemon,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { name } => cmd_init(name).await,
        Commands::Start { message } => cmd_start(message).await,
        Commands::Chat {
            message,
            session,
            ff,
            sender_name,
            json,
            timeout,
        } => cmd_send(session, &message, ff, sender_name.as_deref(), json, timeout).await,
        Commands::Wait {
            session,
            session_arg,
            timeout,
            since,
            json,
        } => cmd_wait(session.or(session_arg), timeout, since, json).await,
        Commands::Recap {
            session,
            session_arg,
            json,
        } => cmd_recap(session.or(session_arg), json).await,
        Commands::List { json } => cmd_list(json).await,
        Commands::Close {
            session,
            session_arg,
            json,
        } => cmd_close(session.or(session_arg), json).await,
        Commands::Status { json } => cmd_status(json).await,
        Commands::Stop => cmd_stop().await,
        Commands::Daemon => crate::daemon::run_daemon().await,
    }
}

async fn ensure_daemon_running() -> anyhow::Result<(String, u16)> {
    match store::read_daemon_json().await {
        Ok(info) => Ok((info.host, info.port)),
        Err(_) => {
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

            bail!("daemon failed to start within 5 seconds");
        }
    }
}

fn daemon_url(host: &str, port: u16, path: &str) -> String {
    format!("http://{}:{}{}", host, port, path)
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

    let url = daemon_url(host, port, "/api/sessions");
    let resp = reqwest::get(&url).await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;
    let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

    match active.len() {
        0 => bail!("No active sessions. Start one with `chit start`"),
        1 => Ok(active[0].id.clone()),
        _ => {
            let ids: Vec<&str> = active.iter().map(|s| s.id.as_str()).collect();
            bail!(
                "Multiple active sessions: {}. Specify one with `chit {} <session>`",
                ids.join(", "),
                cmd_name
            );
        }
    }
}

async fn cmd_init(name: Option<String>) -> anyhow::Result<()> {
    let chit_dir = std::path::PathBuf::from(".chit");
    tokio::fs::create_dir_all(&chit_dir).await?;

    let config_path = chit_dir.join("config.json");
    if config_path.exists() {
        eprintln!("./.chit/config.json already exists");
    } else {
        let project_name = name.unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "project".to_string())
        });
        let config = json!({ "name": project_name });
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&config)?).await?;
        println!("Created ./.chit/config.json with name: {}", project_name);
    }

    install_opencode_skills().await?;

    Ok(())
}

async fn install_opencode_skills() -> anyhow::Result<()> {
    let opencode_dir = std::path::PathBuf::from(".opencode");
    if !opencode_dir.exists() {
        return Ok(());
    }

    let skill_dir = opencode_dir.join("skills").join("chit");
    tokio::fs::create_dir_all(&skill_dir).await?;

    let skill_path = skill_dir.join("SKILL.md");
    let skill = r#"---
name: chit
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires chit CLI (agent-to-agent messaging tool)
metadata:
  author: chit
  version: "1.0"
---

# chit — Agent-to-Agent Messaging

You have access to `chit`, a CLI tool for communicating with agents in other sessions.

## Commands

- `chit start [message]` — Start a new session (optionally with initial message). Outputs a session ID like `sess_abc12`.
- `chit chat [session] <message>` — Send a message in markdown format. Blocks for a reply by default. Use `--ff` to fire-and-forget.
- `chit wait [session]` — Block until a new message arrives. Use `--timeout <secs>` to set a timeout.
- `chit recap [session]` — View the full conversation transcript.
- `chit close [session]` — Close a session.

## Guidelines

- Format messages in **markdown** — use code blocks with language tags, file references as `path/file:line`, and links where useful.
- Include relevant context: error messages, file paths, stack traces, code snippets.
- Use `chit chat` when you need to ask something or provide information to another agent.
- Use `chit wait` when you're expecting a response.
"#;
    tokio::fs::write(&skill_path, skill).await?;
    println!("Created .opencode/skills/chit/SKILL.md");

    let commands_dir = opencode_dir.join("commands");
    tokio::fs::create_dir_all(&commands_dir).await?;
    let command_path = commands_dir.join("chit.md");
    let command = r#"---
description: Use chit for agent-to-agent messaging - start sessions, send messages, wait for replies, and view transcripts.
---

Run chit commands for agent-to-agent messaging. Use `chit start` to create a session, `chit chat` to send a message, `chit wait` to wait for a reply, or `chit recap` to view a transcript.
"#;
    tokio::fs::write(&command_path, command).await?;
    println!("Created .opencode/commands/chit.md");

    Ok(())
}

async fn cmd_start(message: Option<String>) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");

    let req_body = if let Some(ref msg) = message {
        let sender = store::get_sender_name(None);
        CreateSessionRequest {
            message: Some(msg.clone()),
            sender: Some(sender),
        }
    } else {
        CreateSessionRequest {
            message: None,
            sender: None,
        }
    };

    let resp = client.post(&url).json(&req_body).send().await?;

    let session: CreateSessionResponse = resp.json().await?;
    println!("{}", session.id);

    if message.is_some() {
        cmd_wait(Some(session.id.clone()), None, None, false).await?;
    }

    Ok(())
}

async fn cmd_send(
    session_arg: Option<String>,
    content: &str,
    fire_and_forget: bool,
    sender_override: Option<&str>,
    json_output: bool,
    chat_timeout: Option<u64>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "send").await?;

    let sender = store::get_sender_name(sender_override);
    let client = reqwest::Client::new();
    let url = daemon_url(
        &host,
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
        fail(json_output, &err.error, "SEND_ERROR");
    }

    let msg: SendMessageResponse = resp.json().await?;

    if fire_and_forget {
        if json_output {
            println!("{}", serde_json::to_string(&msg).unwrap());
        }
        return Ok(());
    }

    let mut wait_url = format!(
        "/api/sessions/{}/wait?since={}",
        session_id, msg.id
    );
    if let Some(to) = chat_timeout {
        wait_url = format!("{}&timeout_secs={}", wait_url, to);
    }
    let wait_url = daemon_url(&host, port, &wait_url);
    let wait_resp = client.get(&wait_url).send().await?;
    let result: WaitResponse = wait_resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
        if result.timeout {
            process::exit(2);
        }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!(
            "[timeout after {}s, no reply]",
            result.timeout_after.unwrap_or(0)
        );
        process::exit(2);
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
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "wait").await?;

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(300);
    let wait_timeout = timeout_secs.unwrap_or(default_timeout);
    let since_id = since.unwrap_or(0);

    let client = reqwest::Client::new();
    let url = daemon_url(
        &host,
        port,
        &format!(
            "/api/sessions/{}/wait?since={}&timeout_secs={}",
            session_id, since_id, wait_timeout
        ),
    );

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "WAIT_ERROR");
    }

    let result: WaitResponse = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
        if result.timeout {
            process::exit(2);
        }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!(
            "timeout after {}s, no new messages",
            result.timeout_after.unwrap_or(0)
        );
        process::exit(2);
    } else {
        for msg in &result.messages {
            println!(
                "[{}] {} ({}):\n    {}",
                msg.id,
                msg.sender,
                msg.timestamp.format("%H:%M:%S"),
                msg.content
            );
        }
    }

    Ok(())
}

async fn cmd_recap(session_arg: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "recap").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/recap", session_id));
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "RECAP_ERROR");
    }

    let recap: RecapResponse = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&recap).unwrap());
    } else {
        println!(
            "session: {}  |  created: {}  |  closed: {}",
            recap.session.id,
            recap.session.created_at.format("%Y-%m-%d %H:%M:%S"),
            recap.session.closed,
        );
        println!();

        for msg in &recap.messages {
            println!(
                "[{}] {} ({}):\n    {}\n",
                msg.id,
                msg.sender,
                msg.timestamp.format("%H:%M:%S"),
                msg.content,
            );
        }
    }

    Ok(())
}

async fn cmd_list(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&sessions).unwrap());
    } else if sessions.is_empty() {
        println!("No active sessions");
    } else {
        for s in &sessions {
            let status = if s.closed { "closed" } else { "active" };
            println!("{}  {}  {} msgs", s.id, status, s.message_count);
        }
    }

    Ok(())
}

async fn cmd_close(session_arg: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "close").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.delete(&url).send().await?;

    if resp.status().is_success() {
        let result: CloseSessionResponse = resp.json().await?;
        if json_output {
            println!(
                "{}",
                serde_json::json!({"session_id": session_id, "status": result.status})
            );
        } else {
            println!("Session {}: {}", session_id, result.status);
        }
    } else {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "CLOSE_ERROR");
    }

    Ok(())
}

async fn cmd_status(json_output: bool) -> anyhow::Result<()> {
    match store::read_daemon_json().await {
        Ok(info) => {
            if json_output {
                println!("{}", serde_json::to_string(&info).unwrap());
            } else {
                println!("daemon running:");
                println!("  PID:  {}", info.pid);
                println!("  Port: {}", info.port);
                println!("  Host: {}", info.host);
                println!("  Since: {}", info.started_at.format("%Y-%m-%d %H:%M:%S"));
            }
            Ok(())
        }
        Err(_) => {
            if json_output {
                println!("{}", serde_json::json!({"running": false}));
            } else {
                println!("no daemon running");
            }
            Ok(())
        }
    }
}

async fn cmd_stop() -> anyhow::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = store::read_daemon_json().await;
        bail!("stop is not supported on this platform");
    }

    #[cfg(unix)]
    {
        let info = store::read_daemon_json().await?;
        use std::process::Command;
        Command::new("kill")
            .arg(info.pid.to_string())
            .status()
            .context("failed to send SIGTERM")?;

        for _ in 0..20 {
            if store::read_daemon_json().await.is_err() {
                println!("daemon stopped");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        bail!("daemon did not stop in time");
    }
}
